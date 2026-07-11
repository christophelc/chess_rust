use std::collections::HashMap;

use burn::{
    config::Config,
    module::Module,
    nn::{Linear, LinearConfig},
    tensor::{
        activation::relu,
        backend::Backend,
        Tensor,
    },
};

use crate::entity::game::component::square;

#[derive(Module, Debug)]
pub struct KrkModel<B: Backend> {
    input_layer: Linear<B>,
    hidden_layer: Linear<B>,
    output_layer: Linear<B>,
}

#[derive(Config, Debug)]
pub struct KrkModelConfig {
    #[config(default = 128)]
    hidden_size: usize,

    #[config(default = 64)]
    second_hidden_size: usize,
}

impl KrkModelConfig {
    pub fn init<B: Backend>(&self, device: &B::Device) -> KrkModel<B> {
        KrkModel {
            input_layer: LinearConfig::new(192, self.hidden_size)
                .init(device),

            hidden_layer: LinearConfig::new(
                self.hidden_size,
                self.second_hidden_size,
            )
            .init(device),

            output_layer: LinearConfig::new(
                self.second_hidden_size,
                2,
            )
            .init(device),
        }
    }
}

impl<B: Backend> KrkModel<B> {
    pub fn forward(&self, input: Tensor<B, 2>) -> Tensor<B, 2> {
        let x = self.input_layer.forward(input);
        let x = relu(x);

        let x = self.hidden_layer.forward(x);
        let x = relu(x);

        self.output_layer.forward(x)
    }
}

#[derive(Clone, Debug)]
pub struct KrkItem {
    pub features: [f32; 192],
    pub target: [f32; 2], // [king, rook]
    pub mat_in: u8,
}

pub const MOVE_KING: usize = 0;
pub const MOVE_ROOK: usize = 1;

pub fn create_item(
    features: [f32; 192],
    probs: HashMap<square::TypePiece, f32>,
    mat_in: u8,
) -> KrkItem {
    let prob_king = probs.get(&square::TypePiece::King).unwrap_or(&0.0);
    let prob_rook = probs.get(&square::TypePiece::Rook).unwrap_or(&0.0);

    KrkItem {
        features,
        target: [*prob_king, *prob_rook],
        mat_in,
    }
}