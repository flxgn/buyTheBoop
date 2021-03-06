use super::{Amount, Asset, Assets, Exchange, ExchangeOptions, MarketOrder, OrderType};
use crate::{
    messaging::message::{Msg, MsgData, MsgMetaData, PriceUpdated},
    tools::time::{TimeProvider, TimeProviderImpl},
};
use anyhow::Result;
use async_trait::async_trait;
use serde::Deserialize;
use std::{collections::HashMap, fs};
use uuid::Uuid;

pub type Price = f64;

#[derive(Default, Clone)]
pub struct ExchangeSimulation {
    event_stream: Vec<Msg>,
    assets: Assets,
    prices: HashMap<Uuid, Price>,
    options: ExchangeOptions,
}

#[derive(Deserialize)]
struct Candle {
    time: u128,
    close: f64,
}

impl ExchangeSimulation {
    pub fn new(event_stream: Vec<Msg>, assets: Assets, options: ExchangeOptions) -> Self {
        let mut prices = HashMap::new();
        for event in &event_stream {
            if let MsgData::LivePriceUpdated(price_updated) = &event.data {
                prices.insert(event.metadata.correlation_id, price_updated.price);
            }
        }
        ExchangeSimulation {
            event_stream,
            assets,
            prices,
            options,
        }
    }

    pub fn new_from_file(
        file: &str,
        starting_quote: Asset,
        options: ExchangeOptions,
    ) -> ExchangeSimulation {
        let mut time = TimeProviderImpl {};
        let file = fs::File::open(file).expect("file should open read only");
        let candles: Vec<Candle> =
            serde_json::from_reader(file).expect("file should be proper JSON");

        let mut event_stream = vec![];
        for candle in candles {
            let message_id = Uuid::new_v4();
            event_stream.push(Msg {
                data: MsgData::LivePriceUpdated(PriceUpdated {
                    datetime: candle.time,
                    pair_id: "BTC/USDT",
                    price: candle.close,
                }),
                metadata: MsgMetaData {
                    id: message_id,
                    correlation_id: message_id,
                    causation_id: message_id,
                    creation_time: time.now(),
                    correlation_time: candle.time,
                    correlation_price: candle.close,
                },
            });
        }
        event_stream.push(Msg {
            data: MsgData::Shutdown,
            metadata: MsgMetaData {
                ..Default::default()
            },
        });
        ExchangeSimulation::new(event_stream, Assets {
            quote: Some(starting_quote),
            base: None,
        }, options)
    }
}

#[async_trait]
impl Exchange for ExchangeSimulation {
    async fn event_stream(&self) -> Box<dyn Iterator<Item = Msg>> {
        Box::new(self.event_stream.clone().into_iter())
    }

    async fn place_market_order(&mut self, order: &MarketOrder) -> Result<Amount> {
        let price = self
            .prices
            .get(&order.correlation_id)
            .expect("unknown correlation id");
        let amount = order.amount * (1.0 - self.options.fee);
        match order.order_type {
            OrderType::Buy => {
                let amount = if price > &0.0 { amount / price } else { 0.0 };
                self.assets.quote = Some(Asset {
                    name: "USDT".into(),
                    amount: 0.0,
                });
                self.assets.base = Some(Asset {
                    name: "BTC".into(),
                    amount,
                });
                Ok(amount)
            }
            OrderType::Sell => {
                let amount = amount * price;
                self.assets.quote = Some(Asset {
                    name: "USDT".into(),
                    amount,
                });
                self.assets.base = Some(Asset {
                    name: "BTC".into(),
                    amount: 0.0,
                });
                Ok(amount)
            }
        }
    }

    async fn fetch_assets(&self) -> Result<Assets> {
        Ok(self.assets.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exchange::{Asset, OrderType};
    use crate::messaging::message::{Msg, MsgData, MsgMetaData, PriceUpdated};
    use pretty_assertions::assert_eq;

    #[async_std::test]
    async fn event_stream_should_return_given_events() {
        let expected_stream = vec![Msg::with_data(MsgData::LivePriceUpdated(PriceUpdated {
            pair_id: "1",
            ..Default::default()
        }))];
        let exchange = ExchangeSimulation::new(
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
        let exchange = ExchangeSimulation::new(
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
    async fn fetch_assets_should_return_given_assets() {
        let expected_assets = Assets {
            quote: Some(Asset {
                amount: 40.0,
                name: "USDT".into(),
            }),
            base: None,
        };
        let exchange = ExchangeSimulation::new(
            vec![],
            expected_assets.clone(),
            ExchangeOptions {
                ..Default::default()
            },
        );
        let actual_assets = exchange.fetch_assets().await.unwrap();
        assert_eq!(expected_assets, actual_assets)
    }

    #[async_std::test]
    async fn fetch_assets_should_return_different_given_assets() {
        let expected_assets = Assets {
            quote: None,
            base: Some(Asset {
                amount: 0.00001,
                name: "BTC".into(),
            }),
        };
        let exchange = ExchangeSimulation::new(
            vec![],
            expected_assets.clone(),
            ExchangeOptions {
                ..Default::default()
            },
        );
        let actual_assets = exchange.fetch_assets().await.unwrap();
        assert_eq!(expected_assets, actual_assets)
    }

    #[async_std::test]
    async fn place_market_order_should_return_bought_amount() {
        let message_id = Uuid::from_u128(0);
        let mut exchange = ExchangeSimulation::new(
            vec![Msg {
                data: MsgData::LivePriceUpdated(PriceUpdated {
                    pair_id: "BTC/USDT",
                    price: 1.0,
                    ..Default::default()
                }),
                metadata: MsgMetaData {
                    correlation_id: message_id,
                    ..Default::default()
                },
            }],
            Assets {
                quote: Some(Asset {
                    amount: 40.0,
                    name: "USDT".into(),
                }),
                base: None,
            },
            ExchangeOptions {
                ..Default::default()
            },
        );
        let order = MarketOrder {
            base: "BTC".into(),
            quote: "USDT".into(),
            amount: 40.0,
            order_type: OrderType::Buy,
            correlation_id: message_id,
            ..Default::default()
        };
        let actual_amount = exchange.place_market_order(&order).await.unwrap();
        assert_eq!(40.0, actual_amount)
    }

    #[async_std::test]
    async fn place_market_order_should_return_different_bought_amount() {
        let message_id = Uuid::from_u128(0);
        let mut exchange = ExchangeSimulation::new(
            vec![Msg {
                data: MsgData::LivePriceUpdated(PriceUpdated {
                    pair_id: "BTC/USDT",
                    price: 2.0,
                    ..Default::default()
                }),
                metadata: MsgMetaData {
                    correlation_id: message_id,
                    ..Default::default()
                },
            }],
            Assets {
                quote: Some(Asset {
                    amount: 40.0,
                    name: "USDT".into(),
                }),
                base: None,
            },
            ExchangeOptions {
                ..Default::default()
            },
        );

        let order = MarketOrder {
            base: "BTC".into(),
            quote: "USDT".into(),
            amount: 40.0,
            order_type: OrderType::Buy,
            correlation_id: message_id,
            ..Default::default()
        };
        let actual_amount = exchange.place_market_order(&order).await.unwrap();
        assert_eq!(20.0, actual_amount)
    }

    #[async_std::test]
    async fn place_market_order_should_return_sold_amount() {
        let message_id = Uuid::from_u128(0);
        let mut exchange = ExchangeSimulation::new(
            vec![Msg {
                data: MsgData::LivePriceUpdated(PriceUpdated {
                    pair_id: "BTC/USDT",
                    price: 2.0,
                    ..Default::default()
                }),
                metadata: MsgMetaData {
                    correlation_id: message_id,
                    ..Default::default()
                },
            }],
            Assets {
                quote: None,
                base: Some(Asset {
                    amount: 40.0,
                    name: "BTC".into(),
                }),
            },
            ExchangeOptions {
                ..Default::default()
            },
        );

        let order = MarketOrder {
            base: "BTC".into(),
            quote: "USDT".into(),
            amount: 40.0,
            order_type: OrderType::Sell,
            correlation_id: message_id,
            ..Default::default()
        };
        let actual_amount = exchange.place_market_order(&order).await.unwrap();
        assert_eq!(80.0, actual_amount)
    }

    #[async_std::test]
    async fn place_market_order_should_return_sold_amount_deducting_fees() {
        let mut exchange = ExchangeSimulation::new(
            vec![Msg {
                data: MsgData::LivePriceUpdated(PriceUpdated {
                    pair_id: "BTC/USDT",
                    price: 1.0,
                    ..Default::default()
                }),
                metadata: MsgMetaData {
                    ..Default::default()
                },
            }],
            Assets {
                quote: None,
                base: Some(Asset {
                    amount: 40.0,
                    name: "BTC".into(),
                }),
            },
            ExchangeOptions {
                fee: 0.1,
                ..Default::default()
            },
        );

        let order = MarketOrder {
            base: "BTC".into(),
            quote: "USDT".into(),
            amount: 40.0,
            order_type: OrderType::Sell,
            ..Default::default()
        };
        let actual_amount = exchange.place_market_order(&order).await.unwrap();
        assert_eq!(36.0, actual_amount)
    }

    #[async_std::test]
    async fn place_market_order_should_return_different_sold_amount_deducting_fees() {
        let mut exchange = ExchangeSimulation::new(
            vec![Msg {
                data: MsgData::LivePriceUpdated(PriceUpdated {
                    pair_id: "BTC/USDT",
                    price: 0.5,
                    ..Default::default()
                }),
                metadata: MsgMetaData {
                    ..Default::default()
                },
            }],
            Assets {
                quote: None,
                base: Some(Asset {
                    amount: 40.0,
                    name: "BTC".into(),
                }),
            },
            ExchangeOptions {
                fee: 0.2,
                ..Default::default()
            },
        );

        let order = MarketOrder {
            base: "BTC".into(),
            quote: "USDT".into(),
            amount: 40.0,
            order_type: OrderType::Sell,
            ..Default::default()
        };
        let actual_amount = exchange.place_market_order(&order).await.unwrap();
        assert_eq!(16.0, actual_amount)
    }

    #[async_std::test]
    async fn place_market_order_should_return_bought_amount_with_multiple_prices() {
        let message_id = Uuid::from_u128(0);
        let mut exchange = ExchangeSimulation::new(
            vec![
                Msg {
                    data: MsgData::LivePriceUpdated(PriceUpdated {
                        pair_id: "BTC/USDT",
                        price: 0.5,
                        ..Default::default()
                    }),
                    metadata: MsgMetaData {
                        correlation_id: message_id,
                        ..Default::default()
                    },
                },
                Msg {
                    data: MsgData::LivePriceUpdated(PriceUpdated {
                        pair_id: "BTC/USDT",
                        price: 1.0,
                        ..Default::default()
                    }),
                    metadata: MsgMetaData {
                        correlation_id: Uuid::from_u128(1),
                        ..Default::default()
                    },
                },
            ],
            Assets {
                quote: Some(Asset {
                    amount: 40.0,
                    name: "USDT".into(),
                }),
                base: None,
            },
            ExchangeOptions {
                ..Default::default()
            },
        );

        let order = MarketOrder {
            base: "BTC".into(),
            quote: "USDT".into(),
            amount: 40.0,
            order_type: OrderType::Buy,
            correlation_id: message_id,
            ..Default::default()
        };
        let actual_amount = exchange.place_market_order(&order).await.unwrap();
        assert_eq!(80.0, actual_amount)
    }

    #[async_std::test]
    async fn place_market_order_should_update_assets_after_buying() {
        let mut exchange = ExchangeSimulation::new(
            vec![Msg {
                data: MsgData::LivePriceUpdated(PriceUpdated {
                    pair_id: "BTC/USDT",
                    price: 2.0,
                    ..Default::default()
                }),
                metadata: MsgMetaData {
                    ..Default::default()
                },
            }],
            Assets {
                quote: Some(Asset {
                    amount: 40.0,
                    name: "USDT".into(),
                }),
                base: None,
            },
            ExchangeOptions {
                ..Default::default()
            },
        );
        let order = MarketOrder {
            base: "BTC".into(),
            quote: "USDT".into(),
            amount: 40.0,
            order_type: OrderType::Buy,
            ..Default::default()
        };
        exchange.place_market_order(&order).await.unwrap();
        let actual_assets = exchange.fetch_assets().await.unwrap();
        assert_eq!(
            Assets {
                quote: Some(Asset {
                    amount: 0.0,
                    name: "USDT".into(),
                }),
                base: Some(Asset {
                    amount: 20.0,
                    name: "BTC".into(),
                }),
            },
            actual_assets
        )
    }

    #[async_std::test]
    async fn place_market_order_should_update_different_assets_after_buying() {
        let mut exchange = ExchangeSimulation::new(
            vec![Msg {
                data: MsgData::LivePriceUpdated(PriceUpdated {
                    pair_id: "BTC/USDT",
                    price: 1.0,
                    ..Default::default()
                }),
                metadata: MsgMetaData {
                    ..Default::default()
                },
            }],
            Assets {
                quote: Some(Asset {
                    amount: 40.0,
                    name: "USDT".into(),
                }),
                base: None,
            },
            ExchangeOptions {
                ..Default::default()
            },
        );
        let order = MarketOrder {
            base: "BTC".into(),
            quote: "USDT".into(),
            amount: 40.0,
            order_type: OrderType::Buy,
            ..Default::default()
        };
        exchange.place_market_order(&order).await.unwrap();
        let actual_assets = exchange.fetch_assets().await.unwrap();
        assert_eq!(
            Assets {
                quote: Some(Asset {
                    amount: 0.0,
                    name: "USDT".into(),
                }),
                base: Some(Asset {
                    amount: 40.0,
                    name: "BTC".into(),
                }),
            },
            actual_assets
        )
    }

    #[async_std::test]
    async fn place_market_order_should_update_assets_after_selling() {
        let mut exchange = ExchangeSimulation::new(
            vec![Msg {
                data: MsgData::LivePriceUpdated(PriceUpdated {
                    pair_id: "BTC/USDT",
                    price: 2.0,
                    ..Default::default()
                }),
                metadata: MsgMetaData {
                    ..Default::default()
                },
            }],
            Assets {
                quote: None,
                base: Some(Asset {
                    amount: 40.0,
                    name: "BTC".into(),
                }),
            },
            ExchangeOptions {
                ..Default::default()
            },
        );
        let order = MarketOrder {
            base: "BTC".into(),
            quote: "USDT".into(),
            amount: 40.0,
            order_type: OrderType::Sell,
            ..Default::default()
        };
        exchange.place_market_order(&order).await.unwrap();
        let actual_assets = exchange.fetch_assets().await.unwrap();
        assert_eq!(
            Assets {
                quote: Some(Asset {
                    amount: 80.0,
                    name: "USDT".into(),
                }),
                base: Some(Asset {
                    amount: 0.0,
                    name: "BTC".into(),
                }),
            },
            actual_assets
        )
    }

    #[async_std::test]
    async fn place_market_order_should_update_different_assets_after_selling() {
        let mut exchange = ExchangeSimulation::new(
            vec![Msg {
                data: MsgData::LivePriceUpdated(PriceUpdated {
                    pair_id: "BTC/USDT",
                    price: 1.0,
                    ..Default::default()
                }),
                metadata: MsgMetaData {
                    ..Default::default()
                },
            }],
            Assets {
                quote: Some(Asset {
                    amount: 40.0,
                    name: "USDT".into(),
                }),
                base: None,
            },
            ExchangeOptions {
                ..Default::default()
            },
        );
        let order = MarketOrder {
            base: "BTC".into(),
            quote: "USDT".into(),
            amount: 40.0,
            order_type: OrderType::Sell,
            ..Default::default()
        };
        exchange.place_market_order(&order).await.unwrap();
        let actual_assets = exchange.fetch_assets().await.unwrap();
        assert_eq!(
            Assets {
                quote: Some(Asset {
                    amount: 40.0,
                    name: "USDT".into(),
                }),
                base: Some(Asset {
                    amount: 0.0,
                    name: "BTC".into(),
                }),
            },
            actual_assets
        )
    }

    #[async_std::test]
    async fn place_market_order_should_handle_zero_price_for_buying() {
        let mut exchange = ExchangeSimulation::new(
            vec![Msg {
                data: MsgData::LivePriceUpdated(PriceUpdated {
                    pair_id: "BTC/USDT",
                    price: 0.0,
                    ..Default::default()
                }),
                metadata: MsgMetaData {
                    ..Default::default()
                },
            }],
            Assets {
                quote: Some(Asset {
                    amount: 0.0,
                    name: "USDT".into(),
                }),
                base: None,
            },
            ExchangeOptions {
                ..Default::default()
            },
        );
        let order = MarketOrder {
            base: "BTC".into(),
            quote: "USDT".into(),
            amount: 40.0,
            order_type: OrderType::Buy,
            ..Default::default()
        };
        let actual_amount = exchange.place_market_order(&order).await.unwrap();
        assert_eq!(0.0, actual_amount)
    }
}
