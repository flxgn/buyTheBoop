use super::{Assets, Exchange, ExchangeOptions, MarketOrder};
use crate::messaging::message::Msg;
use anyhow::Result;
use async_trait::async_trait;

#[derive(Default)]
pub struct SimulatedExchange {
    event_stream: Vec<Msg>,
}

impl SimulatedExchange {
    pub fn new(event_stream: Vec<Msg>, assets: Assets, options: ExchangeOptions) -> Self {
        SimulatedExchange { event_stream }
    }
}

#[async_trait]
impl Exchange for SimulatedExchange {
    async fn event_stream(&self) -> Box<dyn Iterator<Item = Msg>> {
        Box::new(self.event_stream.clone().into_iter())
    }

    async fn place_market_order(&mut self, order: &MarketOrder) -> Result<MarketOrder> {
        Ok(order.clone())
    }

    async fn fetch_assets(&self) -> Result<Assets> {
        Ok(Assets {
            ..Default::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exchange::{Asset, OrderType};
    use crate::messaging::message::{Msg, MsgData, PriceUpdated};
    use pretty_assertions::assert_eq;

    #[async_std::test]
    async fn event_stream_should_return_given_events() {
        let expected_stream = vec![Msg::with_data(MsgData::LivePriceUpdated(PriceUpdated {
            pair_id: "1",
            ..Default::default()
        }))];
        let exchange = SimulatedExchange::new(
            expected_stream.clone(),
            Assets {
                ..Default::default()
            },
            ExchangeOptions {
                ..Default::default()
            },
        );
        let actual_events: Vec<Msg> = exchange.event_stream().await.collect();
        assert_eq!(expected_stream, actual_events)
    }

    #[async_std::test]
    async fn event_stream_should_return_different_given_events() {
        let expected_stream = vec![Msg::with_data(MsgData::LivePriceUpdated(PriceUpdated {
            pair_id: "2",
            ..Default::default()
        }))];
        let exchange = SimulatedExchange::new(
            expected_stream.clone(),
            Assets {
                ..Default::default()
            },
            ExchangeOptions {
                ..Default::default()
            },
        );
        let actual_events: Vec<Msg> = exchange.event_stream().await.collect();
        assert_eq!(expected_stream, actual_events)
    }

    #[async_std::test]
    async fn place_market_order_should_return_executed_order() {
        let mut exchange = SimulatedExchange::new(
            vec![],
            Assets {
                quote: Some(Asset {
                    amount: 40.0,
                    name: "EUR".into(),
                }),
                base: None,
            },
            ExchangeOptions {
                ..Default::default()
            },
        );
        let expected_order = MarketOrder {
            base: "EUR".into(),
            quote: "BTC".into(),
            amount: 40.0,
            order_type: OrderType::Sell,
        };
        let actual_order = exchange.place_market_order(&expected_order).await.unwrap();
        assert_eq!(expected_order, actual_order)
    }

    // TODO
    #[async_std::test]
    async fn place_market_order_should_return_executed_order_with_fees() {
        let mut exchange = SimulatedExchange::new(
            vec![],
            Assets {
                quote: Some(Asset {
                    amount: 40.0,
                    name: "USDT".into(),
                }),
                base: None,
            },
            ExchangeOptions { fee: 10.0 },
        );
        //TODO Should be own return type becuase returned value will probably be the amount of bought coin
        let expected_order = MarketOrder {
            base: "BTC".into(),
            quote: "USDT".into(),
            amount: 36.0,
            order_type: OrderType::Buy,
        };
        let actual_order = exchange.place_market_order(&expected_order).await.unwrap();
        assert_eq!(expected_order, actual_order)
    }

    // #[async_std::test]
    // async fn fetch_assets_should_return_given_assets() {
    //     assert_eq!(expected_stream, actual_events)
    // }

    // #[async_std::test]
    // async fn fetch_assets_should_return_different_given_assets() {
    //     assert_eq!(expected_stream, actual_events)
    // }

    // #[async_std::test]
    // async fn fetch_assets_should_return_coin_assets_after_market_order_buy() {
    //     assert_eq!(expected_stream, actual_events)
    // }

    // #[async_std::test]
    // async fn fetch_assets_should_return_fiat_assets_after_market_order_sell() {
    //     assert_eq!(expected_stream, actual_events)
    // }
}
