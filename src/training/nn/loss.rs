use burn::tensor::{activation::log_softmax, backend::Backend};

use crate::training::nn::{batch, topology};

pub fn classification_loss<B: Backend>(
    model: &topology::KrkModel<B>,
    batch: batch::KrkBatch<B>,
) -> burn::tensor::Tensor<B, 1> {
    let logits = model.forward(batch.positions);
    let log_probabilities = log_softmax(logits, 1);

    -(batch.targets * log_probabilities)
        .sum_dim(1)
        .mean()
}