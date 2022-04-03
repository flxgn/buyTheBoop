use async_std;
use crossbeam::channel::unbounded;
use exchange::{simulation::ExchangeSimulation, trade::Trader, Asset, Exchange, ExchangeOptions};
use messaging::processor::ActorChain;
use strategy::{crossover::Crossover, sliding_average::SlidingAverage};
use tools::{time::TimeProviderImpl, uuid::UuidProvider};
use view::graph;

mod exchange;
mod messaging;
mod strategy;
mod tools;
mod view;

#[async_std::main]
async fn main() {
    tools::logging::setup();
    let exchange = ExchangeSimulation::new_from_file(
        "example_data_5min_interval.json",
        Asset {
            amount: 1000.0,
            name: "USDT".into(),
        },
        ExchangeOptions {
            fee: 0.0008,
            ..Default::default()
        },
    );

    // TODO: Move exchange into ActorChain as source
    let (sender, in_receiver) = unbounded();
    for event in exchange.event_stream().await {
        sender.send(event).expect("open channel");
    }

    let out_r = ActorChain::new(TimeProviderImpl::new(), UuidProvider::new(), in_receiver)
        .add(SlidingAverage::new(300_000, 1140 * 60 * 1000))
        .add(Crossover::new(0.005))
        .add(Trader::new(exchange))
        .start()
        .await;

    // TODO: Move graph into ActorChain
    graph::draw_graph(out_r, 0.008);
}
