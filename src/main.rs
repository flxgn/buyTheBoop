use crate::messaging::message::{Msg, MsgData};
use async_std;
use chrono::{DateTime, TimeZone, Utc};
use crossbeam::channel;
use crossbeam::channel::unbounded;
use exchange::{simulation::create_simulated_exchange, trade::Trader, Asset, Assets, Exchange};
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
    let out_r = run_simulation(0.008, 1680).await;
    draw_graph(out_r, 0.008)
}

async fn run_enumeration() {
    let mut results = vec![];

    for offset in 1..10 {
        println!("{}", offset);
        for averageing_window in (60..1800).step_by(60) {
            let offset = f64::from(offset) * 0.001;
            let out_receiver = run_simulation(offset, averageing_window).await;

            let mut assets = Assets {
                quote: Some(Asset {
                    name: "USDT".into(),
                    amount: 1000.0,
                }),
                ..Default::default()
            };
            let mut latest_price = 0.0;
            for event in out_receiver.iter() {
                match event.data {
                    MsgData::LivePriceUpdated(price) => {
                        latest_price = price.price;
                    }
                    MsgData::Bought(order) => {
                        assets = Assets {
                            quote: None,
                            base: Some(Asset {
                                name: "BTC".into(),
                                amount: order.amount,
                            }),
                        };
                    }
                    MsgData::Sold(order) => {
                        assets = Assets {
                            quote: Some(Asset {
                                name: "USDT".into(),
                                amount: order.amount,
                            }),
                            base: None,
                        };
                    }
                    _ => (),
                }
            }
            let base_amount = assets.base.clone().unwrap_or_default().amount;
            let quote_amount = assets.quote.clone().unwrap_or_default().amount;
            results.push((
                f64::max(latest_price * base_amount, quote_amount),
                offset,
                averageing_window,
            ));
        }
    }
    results.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    println!("{:#?}", results);
}

async fn run_simulation(offset: f64, averaging_window_minutes: u128) -> channel::Receiver<Msg> {
    let exchange = create_simulated_exchange(0.001, "data_5min.json");

    let (sender, in_receiver) = unbounded();
    for event in exchange.event_stream().await {
        sender.send(event).expect("open channel");
    }

    ActorChain::new(TimeProviderImpl::new(), UuidProvider::new(), in_receiver)
        .add(SlidingAverage::new(averaging_window_minutes * 60 * 1000))
        .add(SimpleCrossover::new(offset))
        .add(Trader::new(exchange))
        .start()
        .await
}

fn draw_graph(out_receiver: channel::Receiver<Msg>, offset: f64) {
    let mut data: Vec<(DateTime<Utc>, f64)> = vec![];
    let mut data_avg: Vec<(DateTime<Utc>, f64)> = vec![];
    let mut data_buys: Vec<(DateTime<Utc>, f64)> = vec![];
    let mut data_sells: Vec<(DateTime<Utc>, f64)> = vec![];
    let mut data_wealth: Vec<(DateTime<Utc>, f64)> = vec![];
    let mut data_base_line: Vec<(DateTime<Utc>, f64)> = vec![];
    let mut base_line_amount = 0.0;
    let mut max_value_price = 0.0;
    let mut min_value_price = f64::INFINITY;
    let mut max_value_weatlh = 0.0;
    let mut min_value_wealth = f64::INFINITY;
    let mut assets = Assets {
        quote: Some(Asset {
            name: "USDT".into(),
            amount: 1000.0,
        }),
        ..Default::default()
    };
    for event in out_receiver.iter() {
        match event.data {
            MsgData::LivePriceUpdated(price) => {
                if base_line_amount == 0.0 {
                    base_line_amount = 1000.0 / price.price;
                }
                if price.price > max_value_price {
                    max_value_price = price.price;
                }
                if price.price < min_value_price {
                    min_value_price = price.price
                }
                let base_amount = assets.base.clone().unwrap_or_default().amount;
                let quote_amount = assets.quote.clone().unwrap_or_default().amount;
                let timestamp = DateTime::from_utc(
                    Utc.timestamp_millis(price.datetime as i64).naive_local(),
                    Utc,
                );

                let current_wealth = f64::max(price.price * base_amount, quote_amount);
                let baseline_wealth = price.price * base_line_amount;

                if current_wealth > max_value_weatlh {
                    max_value_weatlh = current_wealth;
                }
                if baseline_wealth > max_value_weatlh {
                    max_value_weatlh = baseline_wealth;
                }
                if current_wealth < min_value_wealth {
                    min_value_wealth = current_wealth
                }
                if baseline_wealth < min_value_wealth {
                    min_value_wealth = baseline_wealth
                }
                data_wealth.push((timestamp, current_wealth));
                data_base_line.push((timestamp, price.price * base_line_amount));

                data.push((timestamp, price.price));
            }
            MsgData::AveragePriceUpdated(price) => {
                data_avg.push((
                    DateTime::from_utc(
                        Utc.timestamp_millis(price.datetime as i64).naive_local(),
                        Utc,
                    ),
                    price.price,
                ));
            }
            MsgData::Bought(order) => {
                assets = Assets {
                    quote: None,
                    base: Some(Asset {
                        name: "BTC".into(),
                        amount: order.amount,
                    }),
                };
                data_buys.push((
                    DateTime::from_utc(
                        Utc.timestamp_millis(event.metadata.correlation_time as i64)
                            .naive_local(),
                        Utc,
                    ),
                    event.metadata.correlation_price,
                ))
            }
            MsgData::Sold(order) => {
                assets = Assets {
                    quote: Some(Asset {
                        name: "USDT".into(),
                        amount: order.amount,
                    }),
                    base: None,
                };
                data_sells.push((
                    DateTime::from_utc(
                        Utc.timestamp_millis(event.metadata.correlation_time as i64)
                            .naive_local(),
                        Utc,
                    ),
                    event.metadata.correlation_price,
                ))
            }
            _ => (),
        }
    }

    let root_area = SVGBackend::new("images/2.11.svg", (3600, 800)).into_drawing_area();
    root_area.fill(&WHITE).unwrap();
    let (upper, lower) = root_area.split_vertically((70).percent());

    let start_date = data.first().unwrap().0;
    let end_date = data.last().unwrap().0;

    let mut upper_chart = ChartBuilder::on(&upper)
        .set_label_area_size(LabelAreaPosition::Left, 70)
        .set_label_area_size(LabelAreaPosition::Bottom, 40)
        .build_cartesian_2d(start_date..end_date, min_value_price..max_value_price)
        .unwrap();
    upper_chart.configure_mesh().draw().unwrap();

    upper_chart
        .draw_series(data_buys.iter().map(|point| Circle::new(*point, 3, &GREEN)))
        .unwrap();
    upper_chart
        .draw_series(data_sells.iter().map(|point| Circle::new(*point, 3, &RED)))
        .unwrap();

    upper_chart
        .draw_series(LineSeries::new(data.into_iter(), &BLUE))
        .unwrap();
    upper_chart
        .draw_series(LineSeries::new(data_avg.clone().into_iter(), &BLACK))
        .unwrap();
    upper_chart
        .draw_series(LineSeries::new(
            data_avg
                .clone()
                .into_iter()
                .map(|(time, price)| (time, price * (1.0 - offset))),
            &BLACK,
        ))
        .unwrap();
    upper_chart
        .draw_series(LineSeries::new(
            data_avg
                .clone()
                .into_iter()
                .map(|(time, price)| (time, price * (1.0 + offset))),
            &BLACK,
        ))
        .unwrap();

    let mut lower_chart = ChartBuilder::on(&lower)
        .set_label_area_size(LabelAreaPosition::Left, 70)
        .set_label_area_size(LabelAreaPosition::Bottom, 40)
        .build_cartesian_2d(start_date..end_date, min_value_wealth..max_value_weatlh)
        .unwrap();
    lower_chart.configure_mesh().draw().unwrap();
    lower_chart
        .draw_series(LineSeries::new(data_wealth.into_iter(), &RED))
        .unwrap();
    lower_chart
        .draw_series(LineSeries::new(data_base_line.into_iter(), &BLUE))
        .unwrap();
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn run() {}
}
