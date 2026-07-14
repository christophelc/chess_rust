use burn::tensor::{activation::log_softmax, backend::Backend};

use crate::training::nn::{batch, krk_skill::ClassificationOutput, topology};

pub fn classification_output<B: Backend>(
    model: &topology::KrkModel<B>,
    batch: batch::KrkBatch<B>,
) -> ClassificationOutput<B> {
    // Forme : [batch_size, 2]
    let logits = model.forward(batch.positions);

    // Cross-entropy avec targets one-hot.
    let log_probabilities = log_softmax(logits.clone(), 1);

    let loss = -(batch.targets.clone() * log_probabilities)
        .sum_dim(1)
        .mean();

    // Infered class :
    // 0 = king
    // 1 = rook
    //
    // Shape after argmax : [batch_size, 1]
    let predicted_classes = logits.argmax(1);

    // Conversion from one-hot to class index
    let target_classes = batch.targets.argmax(1);

    // % of right predictions.
    let accuracy = predicted_classes
        .clone()
        .equal(target_classes.clone())
        .float()
        .mean();

    // predicted_classes == 0 give predicted positions "rook".
    let predicted_rook_ratio = predicted_classes
        .equal_elem(1)
        .float()
        .mean();

    // real proportionof labeled positions "rook".
    let target_rook_ratio = target_classes
        .equal_elem(1)
        .float()
        .mean();

    ClassificationOutput {
        loss,
        accuracy,
        target_rook_ratio,
        predicted_rook_ratio,
    }
}