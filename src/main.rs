use crate::messaging::message::MsgData;
use async_std;
use chrono::{TimeZone, Utc, DateTime};
use crossbeam::channel::unbounded;
use exchange::{simulation::create_simulated_exchange, trade::Trader, Exchange};
use messaging::processor::ActorChain;
use plotters::prelude::*;
use strategy::{simple_crossover::SimpleCrossover, sliding_average::SlidingAverage};
use tools::{time::TimeProviderImpl, uuid::UuidProvider};

mod exchange;
mod messaging;
mod strategy;
mod tools;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate simple_error;

#[async_std::main]
async fn main() {
    tools::logging::setup();

    let exchange = create_simulated_exchange();

    let (sender, in_receiver) = unbounded();
    for event in exchange.event_stream().await {
        sender.send(event).expect("open channel");
    }

    let out_receiver = ActorChain::new(TimeProviderImpl::new(), UuidProvider::new(), in_receiver)
        .add(SlidingAverage::new(3 * 60 * 60 * 1000))
        .add(SimpleCrossover::new())
        .add(Trader::new(exchange))
        .start()
        .await;

    let mut data: Vec<(DateTime<Utc>, f64)> = vec![];
    let mut data_avg: Vec<(DateTime<Utc>, f64)> = vec![];
    let mut data_buys: Vec<(DateTime<Utc>, f64)> = vec![];
    let mut data_sells: Vec<(DateTime<Utc>, f64)> = vec![];
    let mut max_value = 0.0;
    let mut min_value = f64::INFINITY;
    for event in out_receiver.into_iter() {
        match event.data {
            MsgData::LivePriceUpdated(price) => {
                if price.price > max_value {
                    max_value = price.price;
                }
                if price.price < min_value {
                    min_value = price.price
                }
                data.push((
                    DateTime::from_utc(Utc.timestamp_millis(price.datetime as i64).naive_local(), Utc),
                    price.price,
                ));
            }
            MsgData::AveragePriceUpdated(price) => {
                data_avg.push((
                    DateTime::from_utc(Utc.timestamp_millis(price.datetime as i64).naive_local(), Utc),
                    price.price,
                ));
            },
            MsgData::Bought(_) => {
                data_buys.push((
                    DateTime::from_utc(Utc.timestamp_millis(event.metadata.correlation_time as i64).naive_local(), Utc),
                    event.metadata.correlation_price
                ))
            },
            MsgData::Sold(_) => {
                data_sells.push((
                    DateTime::from_utc(Utc.timestamp_millis(event.metadata.correlation_time as i64).naive_local(), Utc),
                    event.metadata.correlation_price
                ))
            }
            _ => (),
        }
    }
    
    let root_area = SVGBackend::new("images/2.11.svg", (10800, 800)).into_drawing_area();
    root_area.fill(&WHITE).unwrap();

    let start_date = data.first().unwrap().0;
    let end_date = data.last().unwrap().0;
    let mut ctx = ChartBuilder::on(&root_area)
        .set_label_area_size(LabelAreaPosition::Left, 70)
        .set_label_area_size(LabelAreaPosition::Bottom, 40)
        .caption("Liveprice", ("sans-serif", 40))
        .build_cartesian_2d(start_date..end_date, min_value..max_value)
        .unwrap();
    ctx.configure_mesh().draw().unwrap();

    ctx.draw_series(data_buys.iter().map(|point| Circle::new(*point, 3, &GREEN))).unwrap();
    ctx.draw_series(data_sells.iter().map(|point| Circle::new(*point, 3, &RED))).unwrap();
    ctx.draw_series(LineSeries::new(data.into_iter(), &BLUE))
        .unwrap();
    ctx.draw_series(LineSeries::new(data_avg.into_iter(), &BLACK))
        .unwrap();
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn run() {}
}
