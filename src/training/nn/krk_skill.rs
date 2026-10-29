use std::collections::HashMap;
use std::sync::Arc;
use std::{fs::File, io::BufReader};

use crate::training::data_loader::fen_loader;
use crate::training::nn::loss::classification_output;
use crate::training::nn::{batch, loss, topology};
use crate::training::nn::data_split;
use crate::training::nn::dataset;
use std::fs;
use std::io::Write;

use burn::Tensor;
use burn::data::dataloader::{DataLoader, DataLoaderBuilder};
use burn::optim::{AdamConfig, GradientsParams, Optimizer};
use burn::record::CompactRecorder;
use burn::tensor::activation::softmax;
use burn::tensor::backend::{AutodiffBackend, Backend, BackendTypes};
use burn::tensor::{ElementConversion, TensorData};
use burn::module::{AutodiffModule, Module};

type TrainingBackend = burn::backend::Autodiff<burn::backend::Wgpu>;
type InferenceBackend = <TrainingBackend as AutodiffBackend>::InnerBackend;

pub struct ClassificationOutput<B: Backend> {
    pub loss: Tensor<B, 1>,
    pub accuracy: Tensor<B, 1>,
    pub target_rook_ratio: Tensor<B, 1>,
    pub predicted_rook_ratio: Tensor<B, 1>,
}

fn read_krk_positions(filename: &str) ->std::io::Result<fen_loader::FenLineMap> {
    let in_file = File::open(filename).unwrap_or_else(|_| panic!("Cannot open file {}", filename));
    let reader = BufReader::new(in_file);
    fen_loader::read_fens_as_map(reader)
}

fn extract_mat_in_n(data: fen_loader::FenLineMap, mat_in_min: u8, mat_in_max: u8) -> fen_loader::FenLineMap {
    let mut m: fen_loader::FenLineMap = HashMap::new();
    for (k, fen_line_enriched) in data.iter() {
        if let Some(mat_in) = fen_line_enriched.fen_line().mat_in() {
            if mat_in >= mat_in_min && mat_in <= mat_in_max {
                m.insert(k.clone(), fen_line_enriched.clone());    
            }
        }
    }
    m
}

fn create_train_loader<B>(fen_lines_map: fen_loader::FenLineMap, seed: u64) -> Arc<dyn DataLoader<B, batch::KrkBatch<B>>> 
where B: AutodiffBackend {

   let dataset: dataset::KrkDataset = fen_loader::fens_to_dataset(fen_lines_map);
   let dataset = dataset::KrkDataset::new(dataset.items);
    DataLoaderBuilder::new(batch::KrkBatcher)
            .batch_size(128)
            .shuffle(seed)
            .num_workers(1)
            .build(dataset)
}
fn create_loader<B>(fen_lines_map: fen_loader::FenLineMap) -> Arc<dyn DataLoader<B, batch::KrkBatch<B>>> 
where B: Backend {

   let dataset: dataset::KrkDataset = fen_loader::fens_to_dataset(fen_lines_map);
   let dataset = dataset::KrkDataset::new(dataset.items);
    DataLoaderBuilder::new(batch::KrkBatcher)
            .batch_size(128)
            .num_workers(1)
            .build(dataset)
}

fn run(epochs: i32) {
    let seed = 123;
    let mat_in_min = 1;
    let mat_in_max = 3;

    let fens_map = read_krk_positions("database/krk.csv").expect("Cannot read krk.csv");
    tracing::debug!("Filter data...");        
    let fens_map = extract_mat_in_n(fens_map, mat_in_min, mat_in_max);
    tracing::debug!("Filter data done");            
    tracing::debug!("Splitting data...");    
    let split_data: data_split::DataSplit = data_split::split(fens_map, seed);
    tracing::debug!("Splitting data done.");

    let device = Default::default();
    TrainingBackend::seed(&device, seed);

    tracing::debug!("Creating datasets...");
    let train_loader = create_train_loader::<TrainingBackend>(split_data.train, seed);
    let validation_loader = create_loader::<InferenceBackend>(split_data.validation);
    tracing::debug!("Creating datasets done.");
    train_model(epochs, train_loader, validation_loader, device);

    evaluate_model(split_data.test);
}

fn train_model<B>(
    epochs: i32,
    train_loader: Arc<dyn DataLoader<B, batch::KrkBatch<B>>>,
    validation_loader: Arc<dyn DataLoader<B::InnerBackend, batch::KrkBatch<B::InnerBackend>>>,
    device: B::Device) 
    
    where 
        B: AutodiffBackend,
    {

    let mut model = topology::KrkModelConfig::new()
        .init::<B>(&device);    
    let mut optimizer = AdamConfig::new().init();

    let learning_rate = 1e-3;

    let mut best_validation_loss = f32::INFINITY;
    // save weight
    fs::create_dir_all("models")
        .expect("Cannot create models directory");

    let mut metrics_file =
        std::fs::File::create("models/training_metrics.csv")
            .expect("Cannot create metrics file");
    writeln!(
        metrics_file,
        "epoch,train_loss,validation_loss,train_accuracy,validation_accuracy,\
        train_target_rook_ratio,train_predicted_rook_ratio,\
        validation_target_rook_ratio,validation_predicted_rook_ratio"
    )
    .expect("Cannot write CSV header");            

    for epoch in 1..=epochs {
        tracing::debug!("Training epoch {}", epoch);
        let mut total_train_loss = 0.0_f32;
        let mut train_batch_count = 0usize;

        let mut total_train_accuracy = 0.0_f32;
        let mut total_train_target_rook_ratio = 0.0_f32;
        let mut total_train_predicted_rook_ratio = 0.0_f32;

        for batch in train_loader.iter() {
            let output = classification_output(&model, batch);

            let loss_value = output.loss
                .clone()
                .into_scalar()
                .elem::<f32>();

            let accuracy_value = output.accuracy
                .into_scalar()
                .elem::<f32>();

            let target_rook_ratio = output.target_rook_ratio
                .into_scalar()
                .elem::<f32>();

            let predicted_rook_ratio = output.predicted_rook_ratio
                .into_scalar()
                .elem::<f32>();

            let gradients = output.loss.backward();

            total_train_loss += loss_value;
            total_train_accuracy += accuracy_value;
            total_train_target_rook_ratio += target_rook_ratio;
            total_train_predicted_rook_ratio += predicted_rook_ratio;
            train_batch_count += 1;

            let gradients = GradientsParams::from_grads(
                gradients,
                &model,
            );

            model = optimizer.step(
                learning_rate,
                model,
                gradients,
            );
        }

        let denominator = train_batch_count.max(1) as f32;

        let train_loss =
            total_train_loss / denominator;

        let train_accuracy =
            total_train_accuracy / denominator;

        let train_target_rook_ratio =
            total_train_target_rook_ratio / denominator;

        let train_predicted_rook_ratio =
            total_train_predicted_rook_ratio / denominator;

        ///////////////////
        // validation
        let validation_model = model.valid();

        let mut total_validation_loss = 0.0_f32;
        let mut total_validation_accuracy = 0.0_f32;
        let mut total_validation_target_rook_ratio = 0.0_f32;
        let mut total_validation_predicted_rook_ratio = 0.0_f32;
        let mut validation_batch_count = 0usize;

        for batch in validation_loader.iter() {
            let output = classification_output(
                &validation_model,
                batch,
            );

            total_validation_loss += output.loss
                .into_scalar()
                .elem::<f32>();

            total_validation_accuracy += output.accuracy
                .into_scalar()
                .elem::<f32>();

            total_validation_target_rook_ratio += output.target_rook_ratio
                .into_scalar()
                .elem::<f32>();

            total_validation_predicted_rook_ratio += output.predicted_rook_ratio
                .into_scalar()
                .elem::<f32>();

            validation_batch_count += 1;
        }

        let validation_denominator =
            validation_batch_count.max(1) as f32;

        let validation_loss =
            total_validation_loss / validation_denominator;

        let validation_accuracy =
            total_validation_accuracy / validation_denominator;

        let validation_target_rook_ratio =
            total_validation_target_rook_ratio / validation_denominator;

        let validation_predicted_rook_ratio =
            total_validation_predicted_rook_ratio / validation_denominator;

        writeln!(
            metrics_file,
            "{epoch},{train_loss},{validation_loss},\
            {train_accuracy},{validation_accuracy},\
            {train_target_rook_ratio},{train_predicted_rook_ratio},\
            {validation_target_rook_ratio},{validation_predicted_rook_ratio}"
        )
        .expect("Cannot write metrics");

        // save model weights
        if validation_loss < best_validation_loss {
            best_validation_loss = validation_loss;

            model
                .clone()
                .valid()
                .save_file(
                    "models/krk_model_best",
                    &CompactRecorder::new(),
                )
                .expect("Cannot save best model");

            println!(
                "Best model saved: validation={validation_loss:.6}"
            );
        }

        println!(
            "Epoch {epoch:03} | \
            train loss={train_loss:.6} accuracy={:.2}% \
            targets tour={:.2}% predictions tour={:.2}% | \
            validation loss={validation_loss:.6} accuracy={:.2}% \
            targets tour={:.2}% predictions tour={:.2}%",
            train_accuracy * 100.0,
            train_target_rook_ratio * 100.0,
            train_predicted_rook_ratio * 100.0,
            validation_accuracy * 100.0,
            validation_target_rook_ratio * 100.0,
            validation_predicted_rook_ratio * 100.0,
        );
    }

}

pub struct LoadedKrkModel {
    pub model: topology::KrkModel<InferenceBackend>,
    pub device: <InferenceBackend as BackendTypes>::Device,
}

pub fn load_model() -> LoadedKrkModel {
    let device = Default::default();

    let model = topology::KrkModelConfig::new()
        .init::<InferenceBackend>(&device)
        .load_file(
            "models/krk_model_best",
            &CompactRecorder::new(),
            &device,
        )
        .expect("Cannot load KRK model");
    LoadedKrkModel { model, device }
}

fn evaluate_model(test_fen_lines_map: fen_loader::FenLineMap) {
    // load model
    let loaded = load_model();
    let test_loader = create_loader::<InferenceBackend>(test_fen_lines_map);

    let mut total_test_loss = 0.0_f32;
    let mut total_test_accuracy = 0.0_f32;
    let mut total_test_target_rook_ratio = 0.0_f32;
    let mut total_test_predicted_rook_ratio = 0.0_f32;
    let mut test_batch_count = 0usize;

    for batch in test_loader.iter() {
        let output = loss::classification_output(
            &loaded.model,
            batch,
        );

        total_test_loss += output.loss.into_scalar().elem::<f32>();
        total_test_accuracy += output.accuracy.into_scalar().elem::<f32>();
        total_test_target_rook_ratio += output.target_rook_ratio.into_scalar().elem::<f32>();
        total_test_predicted_rook_ratio += output.predicted_rook_ratio.into_scalar().elem::<f32>();

        test_batch_count += 1;
    }
    assert!(
        test_batch_count > 0,
        "The test dataset is empty"
    );
    let n = test_batch_count as f32;

    let test_loss = total_test_loss / n;
    let test_accuracy = total_test_accuracy / n;
    let test_target_rook_ratio = total_test_target_rook_ratio / n;
    let test_predicted_rook_ratio = total_test_predicted_rook_ratio / n;

    println!(
        "Test | loss={:.6} | accuracy={:.2}% | \
        targets tour={:.2}% | predictions tour={:.2}%",
        test_loss,
        test_accuracy * 100.0,
        test_target_rook_ratio * 100.0,
        test_predicted_rook_ratio * 100.0,
    );
}

pub fn infer_krk<B: Backend>(
    model: &topology::KrkModel<B>,
    fen: &str,
    device: &B::Device,
) -> [f32; 2] {
    let bitboards = fen_loader::fen_krk_to_bitboards(fen);
    let features = fen_loader::bitboards_to_features(bitboards);

    let input = Tensor::<B, 2>::from_data(
        TensorData::new(features.to_vec(), [1, 192]),
        device,
    );

    // [1, 2] : logits [king, rook]
    let logits = model.forward(input);

    // [1, 2] : probabilités
    let probabilities = softmax(logits, 1);

    let values = probabilities
        .into_data()
        .to_vec::<f32>()
        .expect("Cannot convert prediction to f32");

    [values[0], values[1]]
}

#[cfg(test)]
use crate::trace::{init_trace, trace_build_info};

#[actix::test]
async fn test_load_fen_lines_as_map() {
    let fens_map = read_krk_positions("database/krk.csv").expect("Cannot read krk.csv");
    assert!(!fens_map.is_empty());
    let key = fens_map.keys().next();
    assert!(key.is_some());
    let mat_in_one = extract_mat_in_n(fens_map, 1, 1);
    assert_eq!(mat_in_one.len(), 1512);
}

#[test]
fn test_run() {
    init_trace();
    trace_build_info();
    run(100)
}

#[test]
fn test_infer() {
    init_trace();
    trace_build_info();    

    let loaded = load_model();

    let line = "8/8/3k4/8/8/8/8/KR6 w - - 0 0;13;a1a2,b1b4,b1b5,b1h1,b1d1,a1b2";
    let fen_line = fen_loader::extract_fields(line);    
    let result = infer_krk(&loaded.model, fen_line.fen(), &loaded.device);
    assert!(
        result[1] - result[0] > 0.20,
        "Moving rook is far better than moving king: {result:?}"
    );    

    let line = "8/8/8/1k6/8/8/8/K4R2 w - - 0 0;10;f1c1";
    let fen_line = fen_loader::extract_fields(line);        
    let result = infer_krk(&loaded.model, fen_line.fen(), &loaded.device);
    assert!(
        result[1] - result[0] > 0.20,
        "Moving rook is far better than moving king: {result:?}"
    );    

    let line: &str = "8/8/8/2R5/8/2K5/8/k7 w - - 0 1";
    let fen_line = fen_loader::extract_fields(line);        
    let result = infer_krk(&loaded.model, fen_line.fen(), &loaded.device);
    assert!(
        result[0] - result[1] > 0.20,
        "Moving king is far better than moving rook: {result:?}"
    );    

    let line: &str = "8/8/8/8/7R/2K5/8/k7 w - - 0 0;2;c3b3,c3c2";
    let fen_line = fen_loader::extract_fields(line);        
    let result = infer_krk(&loaded.model, fen_line.fen(), &loaded.device);
    assert!(
        result[0] - result[1] > 0.20,
        "Moving king is far better than moving rook: {result:?}"
    );        

}