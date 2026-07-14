use plotters::prelude::*;
use serde::Deserialize;
use std::error::Error;

#[derive(Debug, Deserialize)]
struct MetricRow {
    epoch: u32,
    train_loss: f32,
    validation_loss: f32,

    train_accuracy: f32,
    validation_accuracy: f32,

    train_target_rook_ratio: f32,
    train_predicted_rook_ratio: f32,

    validation_target_rook_ratio: f32,
    validation_predicted_rook_ratio: f32,
}

fn plot_losses(csv_path: &str, output_path: &str) -> Result<(), Box<dyn Error>> {
    let mut reader = csv::Reader::from_path(csv_path)?;

    let rows: Vec<MetricRow> = reader
        .deserialize()
        .collect::<Result<Vec<_>, _>>()?;

    if rows.is_empty() {
        return Err("No metrics found".into());
    }

    let max_epoch = rows
        .iter()
        .map(|row| row.epoch)
        .max()
        .unwrap_or(1);

    let max_loss = rows
        .iter()
        .flat_map(|row| [row.train_loss, row.validation_loss])
        .fold(0.0_f32, f32::max)
        .max(0.01);

    let root = BitMapBackend::new(output_path, (1200, 800))
        .into_drawing_area();

    root.fill(&WHITE)?;

    let mut chart = ChartBuilder::on(&root)
        .caption("KRK training loss", ("sans-serif", 40))
        .margin(20)
        .x_label_area_size(50)
        .y_label_area_size(60)
        .build_cartesian_2d(
            1_u32..max_epoch.max(2),
            0.0_f32..max_loss * 1.1,
        )?;

    chart
        .configure_mesh()
        .x_desc("Epoch")
        .y_desc("Loss")
        .draw()?;

    chart
        .draw_series(LineSeries::new(
            rows.iter().map(|row| (row.epoch, row.train_target_rook_ratio)),
            RED,
        ))?
        .label("Real rook ratio")
        .legend(|(x, y)| PathElement::new([(x, y), (x + 20, y)], RED));

    chart
        .draw_series(LineSeries::new(
            rows.iter().map(|row| (row.epoch, row.train_predicted_rook_ratio)),
            BLUE,
        ))?
        .label("Predicted rook ratio")
        .legend(|(x, y)| PathElement::new([(x, y), (x + 20, y)], BLUE));

    chart
        .configure_series_labels()
        .border_style(BLACK)
        .background_style(WHITE.mix(0.8))
        .draw()?;

    root.present()?;

    println!("Graph generated: {output_path}");

    Ok(())
}

#[cfg(test)]

#[test]
fn test_plot() {
    let _ = plot_losses(
        "models/training_metrics.csv",
        "models/loss.png",
    );    
}
