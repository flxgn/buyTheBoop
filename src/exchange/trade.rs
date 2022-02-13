use crate::messaging::{
    message::MessageId, message::Msg, message::MsgData, message::Order, processor::Actor,
};
use anyhow::Result;
use async_trait::async_trait;

use super::{Asset, Exchange, MarketOrder, OrderType};

#[derive(Debug, PartialEq)]
pub struct Trader<'a, E>
where
    E: Exchange,
{
    exchange: &'a mut E,
}

impl<'a, E> Trader<'a, E>
where
    E: Exchange,
{
    pub fn new(exchange: &'a mut E) -> Self {
        Trader { exchange }
    }
}

#[async_trait]
impl<'a, E> Actor for Trader<'a, E>
where
    E: Exchange + Send + Sync,
{
    async fn act(&mut self, msg: &Msg) -> Result<Vec<MsgData>> {
        let res = match msg.data {
            MsgData::Buy => {
                let assets = self.exchange.fetch_assets().await?;
                execute(
                    self.exchange,
                    assets.quote,
                    OrderType::Buy,
                    msg.metadata.correlation_id,
                )
                .await?
            }
            MsgData::Sell => {
                let assets = self.exchange.fetch_assets().await?;
                execute(
                    self.exchange,
                    assets.base,
                    OrderType::Sell,
                    msg.metadata.correlation_id,
                )
                .await?
            }
            _ => vec![],
        };
        Ok(res)
    }
}

async fn execute<'a, E>(
    exchange: &'a mut E,
    asset: Option<Asset>,
    order_type: OrderType,
    correlation_id: MessageId,
) -> Result<Vec<MsgData>>
where
    E: Exchange,
{
    if let Some(asset) = asset {
        if asset.amount > 0.0 {
            let order = MarketOrder {
                base: "BTC".into(),
                quote: "USDT".into(),
                amount: asset.amount,
                order_type,
                correlation_id,
            };
            return exchange.place_market_order(&order).await.map(|amount| {
                match order.order_type {
                    OrderType::Buy => vec![MsgData::Bought(Order {
                        amount,
                        quote: order.quote,
                        base: order.base,
                    })],
                    OrderType::Sell => vec![MsgData::Sold(Order {
                        amount,
                        quote: order.quote,
                        base: order.base,
                    })],
                }
            });
        }
    }
    Ok(vec![])
}

#[cfg(test)]
mod tests {
    use crate::exchange::{Assets, MockExchange};

    use super::*;
    use crate::messaging::message::MsgMetaData;
    use pretty_assertions::assert_eq;
    use uuid::Uuid;

    #[async_std::test]
    async fn should_buy_max_amount_of_quote() {
        let mut exchange = MockExchange::new(Assets {
            quote: Some(Asset {
                amount: 40.0,
                name: "USDT".into(),
            }),
            base: None,
        });
        let mut trader = Trader::new(&mut exchange);

        trader.act(&Msg::with_data(MsgData::Buy)).await.unwrap();

        let expected = vec![MarketOrder {
            base: "BTC".into(),
            quote: "USDT".into(),
            amount: 40.0,
            order_type: OrderType::Buy,
            ..Default::default()
        }];
        let actual = exchange.recorded_orders;
        assert_eq!(expected, actual)
    }

    #[async_std::test]
    async fn should_buy_different_max_amount_of_quote() {
        let mut exchange = MockExchange::new(Assets {
            quote: Some(Asset {
                amount: 50.0,
                name: "USDT".into(),
            }),
            base: None,
        });
        let mut trader = Trader::new(&mut exchange);

        trader.act(&Msg::with_data(MsgData::Buy)).await.unwrap();

        let expected = vec![MarketOrder {
            base: "BTC".into(),
            quote: "USDT".into(),
            amount: 50.0,
            order_type: OrderType::Buy,
            ..Default::default()
        }];
        let actual = exchange.recorded_orders;
        assert_eq!(expected, actual)
    }

    #[async_std::test]
    async fn should_not_buy_quote_when_no_assets() {
        let mut exchange = MockExchange::new(Assets {
            ..Default::default()
        });
        let mut trader = Trader::new(&mut exchange);

        trader.act(&Msg::with_data(MsgData::Buy)).await.unwrap();

        let expected: Vec<MarketOrder> = vec![];
        let actual = exchange.recorded_orders;
        assert_eq!(expected, actual)
    }

    #[async_std::test]
    async fn should_not_buy_quote_when_zero() {
        let mut exchange = MockExchange::new(Assets {
            quote: Some(Asset {
                amount: 0.0,
                name: "USDT".into(),
            }),
            base: None,
        });
        let mut trader = Trader::new(&mut exchange);

        trader.act(&Msg::with_data(MsgData::Buy)).await.unwrap();

        let expected: Vec<MarketOrder> = vec![];
        let actual = exchange.recorded_orders;
        assert_eq!(expected, actual)
    }

    #[async_std::test]
    async fn should_emit_bought_order_event() {
        let mut exchange = MockExchange::new(Assets {
            quote: Some(Asset {
                amount: 50.0,
                name: "USDT".into(),
            }),
            base: None,
        });
        let mut trader = Trader::new(&mut exchange);

        let actual = trader.act(&Msg::with_data(MsgData::Buy)).await.unwrap();

        let expected = vec![MsgData::Bought(Order {
            base: "BTC".into(),
            quote: "USDT".into(),
            amount: 45.0,
        })];
        assert_eq!(expected, actual)
    }

    #[async_std::test]
    async fn should_sell_max_amount_of_base() {
        let mut exchange = MockExchange::new(Assets {
            quote: None,
            base: Some(Asset {
                amount: 0.0000001,
                name: "BTC".into(),
            }),
        });
        let mut trader = Trader::new(&mut exchange);

        trader.act(&Msg::with_data(MsgData::Sell)).await.unwrap();

        let expected = vec![MarketOrder {
            base: "BTC".into(),
            quote: "USDT".into(),
            amount: 0.0000001,
            order_type: OrderType::Sell,
            ..Default::default()
        }];
        let actual = exchange.recorded_orders;
        assert_eq!(expected, actual)
    }

    #[async_std::test]
    async fn should_sell_different_max_amount_of_base() {
        let mut exchange = MockExchange::new(Assets {
            quote: None,
            base: Some(Asset {
                amount: 0.0002,
                name: "BTC".into(),
            }),
        });
        let mut trader = Trader::new(&mut exchange);

        trader.act(&Msg::with_data(MsgData::Sell)).await.unwrap();

        let expected = vec![MarketOrder {
            base: "BTC".into(),
            quote: "USDT".into(),
            amount: 0.0002,
            order_type: OrderType::Sell,
            ..Default::default()
        }];
        let actual = exchange.recorded_orders;
        assert_eq!(expected, actual)
    }

    #[async_std::test]
    async fn should_not_sell_when_no_assets() {
        let mut exchange = MockExchange::new(Assets {
            ..Default::default()
        });
        let mut trader = Trader::new(&mut exchange);

        trader.act(&Msg::with_data(MsgData::Sell)).await.unwrap();

        let expected: Vec<MarketOrder> = vec![];
        let actual = exchange.recorded_orders;
        assert_eq!(expected, actual)
    }

    #[async_std::test]
    async fn should_not_sell_when_base_zero() {
        let mut exchange = MockExchange::new(Assets {
            quote: None,
            base: Some(Asset {
                amount: 0.0,
                name: "BTC".into(),
            }),
        });
        let mut trader = Trader::new(&mut exchange);

        trader.act(&Msg::with_data(MsgData::Sell)).await.unwrap();

        let expected: Vec<MarketOrder> = vec![];
        let actual = exchange.recorded_orders;
        assert_eq!(expected, actual)
    }

    #[async_std::test]
    async fn should_set_correlation_id() {
        let mut exchange = MockExchange::new(Assets {
            quote: None,
            base: Some(Asset {
                amount: 0.0002,
                name: "BTC".into(),
            }),
        });
        let mut trader = Trader::new(&mut exchange);
        let uuid = Uuid::from_u128(0);

        trader
            .act(&Msg {
                data: MsgData::Sell,
                metadata: MsgMetaData {
                    correlation_id: uuid,
                    ..Default::default()
                },
            })
            .await
            .unwrap();

        let expected = vec![MarketOrder {
            base: "BTC".into(),
            quote: "USDT".into(),
            amount: 0.0002,
            order_type: OrderType::Sell,
            correlation_id: uuid,
            ..Default::default()
        }];
        let actual = exchange.recorded_orders;
        assert_eq!(expected, actual)
    }

    #[async_std::test]
    async fn should_set_different_correlation_id() {
        let mut exchange = MockExchange::new(Assets {
            quote: None,
            base: Some(Asset {
                amount: 0.0002,
                name: "BTC".into(),
            }),
        });
        let mut trader = Trader::new(&mut exchange);
        let uuid = Uuid::from_u128(1);

        trader
            .act(&Msg {
                data: MsgData::Sell,
                metadata: MsgMetaData {
                    correlation_id: uuid,
                    ..Default::default()
                },
            })
            .await
            .unwrap();

        let expected = vec![MarketOrder {
            base: "BTC".into(),
            quote: "USDT".into(),
            amount: 0.0002,
            order_type: OrderType::Sell,
            correlation_id: uuid,
            ..Default::default()
        }];
        let actual = exchange.recorded_orders;
        assert_eq!(expected, actual)
    }

    #[async_std::test]
    async fn should_emit_sold_order_event() {
        let mut exchange = MockExchange::new(Assets {
            quote: None,
            base: Some(Asset {
                amount: 20.0,
                name: "BTC".into(),
            }),
        });
        let mut trader = Trader::new(&mut exchange);

        let actual = trader.act(&Msg::with_data(MsgData::Sell)).await.unwrap();

        let expected = vec![MsgData::Sold(Order {
            base: "BTC".into(),
            quote: "USDT".into(),
            amount: 18.0,
        })];
        assert_eq!(expected, actual)
    }
}
