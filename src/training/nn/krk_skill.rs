use std::sync::Arc;
use std::{fs::File, io::BufReader};

use crate::training::data_loader::fen_loader;
use crate::training::nn::{batch, loss, topology};
use crate::training::nn::data_split;
use crate::training::nn::dataset;
use std::fs;

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


fn read_krk_positions(filename: &str) ->std::io::Result<fen_loader::FenLineMap> {
    let in_file = File::open(filename).unwrap_or_else(|_| panic!("Cannot open file {}", filename));
    let reader = BufReader::new(in_file);
    fen_loader::read_fens_as_map(reader)
}

fn extract_mat_in_n(data: fen_loader::FenLineMap, mat_in_min: u8, mat_in_max: u8) -> Vec<fen_loader::FenLine> {
    let mut v: Vec<fen_loader::FenLine> = vec![];
    for (_k, fen_line_enriched) in data.iter() {
        if let Some(mat_in) = fen_line_enriched.fen_line().mat_in() {
            if mat_in >= mat_in_min && mat_in <= mat_in_max {
                v.push(fen_line_enriched.fen_line().clone());    
            }
        }
    }
    v
}

fn create_train_loader<B>(fen_lines_map: fen_loader::FenLineMap, seed: u64) -> Arc<dyn DataLoader<B, batch::KrkBatch<B>>> 
where B: AutodiffBackend {

   let dataset: dataset::KrkDataset = fen_loader::fens_to_dataset(fen_lines_map);
   let dataset = dataset::KrkDataset::new(dataset.items);
    DataLoaderBuilder::new(batch::KrkBatcher::default())
            .batch_size(128)
            .shuffle(seed)
            .num_workers(1)
            .build(dataset)
}
fn create_loader<B>(fen_lines_map: fen_loader::FenLineMap) -> Arc<dyn DataLoader<B, batch::KrkBatch<B>>> 
where B: Backend {

   let dataset: dataset::KrkDataset = fen_loader::fens_to_dataset(fen_lines_map);
   let dataset = dataset::KrkDataset::new(dataset.items);
    DataLoaderBuilder::new(batch::KrkBatcher::default())
            .batch_size(128)
            .num_workers(1)
            .build(dataset)
}

fn run() {
    let seed = 123;
    let fens_map = read_krk_positions("database/krk.csv").expect("Cannot read krk.csv");
    tracing::debug!("Splitting data...");    
    let split_data: data_split::DataSplit = data_split::split(fens_map, seed);
    tracing::debug!("Splitting data done.");

    let device = Default::default();
    TrainingBackend::seed(&device, seed);

    tracing::debug!("Creating datasets...");
    let train_loader = create_train_loader::<TrainingBackend>(split_data.train, seed);
    let validation_loader = create_loader::<InferenceBackend>(split_data.validation);
    tracing::debug!("Creating datasets done.");
    train_model(train_loader, validation_loader, device);

    evaluate_model(split_data.test);
}

fn train_model<B>(
    train_loader: Arc<dyn DataLoader<B, batch::KrkBatch<B>>>,
    validation_loader: Arc<dyn DataLoader<B::InnerBackend, batch::KrkBatch<B::InnerBackend>>>,
    device: B::Device) 
    
    where 
        B: AutodiffBackend,
    {

    let mut model = topology::KrkModelConfig::new()
        .init::<B>(&device);    
    let mut optimizer = AdamConfig::new().init();

    let epochs = 30;
    let learning_rate = 1e-3;

    let mut best_validation_loss = f32::INFINITY;
    // save weight
    fs::create_dir_all("models")
        .expect("Cannot create models directory");

    for epoch in 1..=epochs {
        tracing::debug!("Training epoch {}", epoch);
        let mut total_train_loss = 0.0_f32;
        let mut train_batch_count = 0usize;

        for batch in train_loader.iter() {
            let loss = loss::classification_loss(
                &model,
                batch,
            );

            total_train_loss += loss.clone().into_scalar().elem::<f32>();
            train_batch_count += 1;

            let gradients = loss.backward();

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

        let train_loss =
            total_train_loss / train_batch_count.max(1) as f32;

        let validation_model = model.valid();

        let mut total_validation_loss = 0.0_f32;
        let mut validation_batch_count = 0usize;

        for batch in validation_loader.iter() {
            let loss = loss::classification_loss(
                &validation_model,
                batch,
            );

            total_validation_loss += loss.into_scalar().elem::<f32>();
            validation_batch_count += 1;
        }

        let validation_loss =
            total_validation_loss
                / validation_batch_count.max(1) as f32;

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
            "Epoch {epoch:03} | train={train_loss:.6} | validation={validation_loss:.6}"
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
            "models/krk_model",
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
    let mut test_batch_count = 0usize;

    for batch in test_loader.iter() {
        let loss = loss::classification_loss(
            &loaded.model,
            batch,
        );

        total_test_loss += loss.into_scalar();
        test_batch_count += 1;
    }
    assert!(
        test_batch_count > 0,
        "The test dataset is empty"
    );
    let test_loss =
        total_test_loss / test_batch_count as f32;

    println!("Final test loss: {test_loss:.6}");    

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
    run()
}

#[test]
fn test_infer() {
    init_trace();
    trace_build_info();    

    let loaded = load_model();

    let line = "8/8/3k4/8/8/8/8/KR6 w - - 0 0;13;a1a2,b1b4,b1b5,b1h1,b1d1,a1b2";
    let fen_line = fen_loader::extract_fields(line);    
    let result = infer_krk(&loaded.model, fen_line.fen(), &loaded.device);
    let d = (result[0] - 1.0/3.0).abs() + (result[1] - 2.0/3.0).abs();
    assert!(d < 0.1);

    let line = "8/8/8/1k6/8/8/8/K4R2 w - - 0 0;10;f1c1";
    let fen_line = fen_loader::extract_fields(line);        
    let result = infer_krk(&loaded.model, fen_line.fen(), &loaded.device);
    let d = result[0].abs() + (result[1] - 1.0).abs();    
    // fail: d ~ 0.8
    assert!(d < 0.1);    
}