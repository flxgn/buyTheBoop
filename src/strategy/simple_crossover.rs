use crate::messaging::message::{Msg, MsgData, Price};
use crate::messaging::processor::Actor;
use anyhow::Result;
use async_trait::async_trait;

#[derive(Debug, PartialEq, Clone, Default)]
pub struct SimpleCrossover {
    latest_average: Option<Price>,
    latest_live: Option<Price>,
}

impl SimpleCrossover {
    pub fn new() -> Self {
        SimpleCrossover {
            ..Default::default()
        }
    }
}

#[async_trait]
impl Actor for SimpleCrossover {
    async fn act(&mut self, msg: &Msg) -> Result<Vec<MsgData>> {
        let res = match &msg.data {
            MsgData::LivePriceUpdated(e) => {
                let result = self
                    .latest_live
                    .and_then(|live| {
                        self.latest_average.map(|avg| {
                            if e.price > avg && live < avg {
                                vec![MsgData::Buy]
                            } else if e.price < avg && live > avg {
                                vec![MsgData::Sell]
                            } else {
                                vec![]
                            }
                        })
                    })
                    .unwrap_or(vec![]);
                self.latest_live = Some(e.price);
                result
            }
            MsgData::AveragePriceUpdated(e) => {
                self.latest_average = Some(e.price);
                vec![]
            }
            _ => vec![],
        };
        Ok(res)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messaging::message::PriceUpdated;
    use pretty_assertions::assert_eq;

    const SECOND: i128 = 1_000;

    #[async_std::test]
    async fn actor_should_emit_nothing_if_only_average_price_updated() {
        let mut aggr = SimpleCrossover::new();
        let msg = Msg::with_data(MsgData::AveragePriceUpdated(PriceUpdated {
            pair_id: "pair_id",
            datetime: 0,
            price: 1.0,
            ..Default::default()
        }));
        let actual = aggr.act(&msg).await.unwrap();
        let expected: Vec<MsgData> = vec![];
        assert_eq!(expected, actual)
    }

    #[async_std::test]
    async fn actor_should_emit_nothing_if_only_live_price_updated() {
        let mut aggr = SimpleCrossover::new();
        let msg = Msg::with_data(MsgData::LivePriceUpdated(PriceUpdated {
            pair_id: "pair_id",
            datetime: 0,
            price: 1.0,
            ..Default::default()
        }));
        let actual = aggr.act(&msg).await.unwrap();
        let expected: Vec<MsgData> = vec![];
        assert_eq!(expected, actual)
    }

    #[async_std::test]
    async fn actor_should_emit_buy_msg_if_live_price_crosses_average_upwards() {
        let mut aggr = SimpleCrossover::new();
        let average_updated = Msg::with_data(MsgData::AveragePriceUpdated(PriceUpdated {
            pair_id: "pair_id",
            datetime: 0,
            price: 1.0,
            ..Default::default()
        }));
        let live_updated_1 = Msg::with_data(MsgData::LivePriceUpdated(PriceUpdated {
            pair_id: "pair_id",
            datetime: SECOND,
            price: 0.5,
            ..Default::default()
        }));
        let live_updated_2 = Msg::with_data(MsgData::LivePriceUpdated(PriceUpdated {
            pair_id: "pair_id",
            datetime: SECOND * 2,
            price: 1.1,
            ..Default::default()
        }));
        aggr.act(&average_updated).await.unwrap();
        aggr.act(&live_updated_1).await.unwrap();
        let actual = aggr.act(&live_updated_2).await.unwrap();
        let expected: Vec<MsgData> = vec![MsgData::Buy];
        assert_eq!(expected, actual)
    }

    #[async_std::test]
    async fn actor_should_emit_nothing_if_live_price_stays_above_average() {
        let mut aggr = SimpleCrossover::new();
        let average_updated = Msg::with_data(MsgData::AveragePriceUpdated(PriceUpdated {
            pair_id: "pair_id",
            datetime: 0,
            price: 1.0,
            ..Default::default()
        }));
        let live_updated_1 = Msg::with_data(MsgData::LivePriceUpdated(PriceUpdated {
            pair_id: "pair_id",
            datetime: SECOND,
            price: 1.1,
            ..Default::default()
        }));
        let live_updated_2 = Msg::with_data(MsgData::LivePriceUpdated(PriceUpdated {
            pair_id: "pair_id",
            datetime: SECOND * 2,
            price: 1.2,
            ..Default::default()
        }));
        aggr.act(&average_updated).await.unwrap();
        aggr.act(&live_updated_1).await.unwrap();
        let actual = aggr.act(&live_updated_2).await.unwrap();
        let expected: Vec<MsgData> = vec![];
        assert_eq!(expected, actual)
    }

    #[async_std::test]
    async fn actor_should_emit_sell_msg_if_live_price_crosses_average_downwards() {
        let mut aggr = SimpleCrossover::new();
        let average_updated = Msg::with_data(MsgData::AveragePriceUpdated(PriceUpdated {
            pair_id: "pair_id",
            datetime: 0,
            price: 1.0,
            ..Default::default()
        }));
        let live_updated_1 = Msg::with_data(MsgData::LivePriceUpdated(PriceUpdated {
            pair_id: "pair_id",
            datetime: SECOND,
            price: 1.1,
            ..Default::default()
        }));
        let live_updated_2 = Msg::with_data(MsgData::LivePriceUpdated(PriceUpdated {
            pair_id: "pair_id",
            datetime: SECOND * 2,
            price: 0.9,
            ..Default::default()
        }));
        aggr.act(&average_updated).await.unwrap();
        aggr.act(&live_updated_1).await.unwrap();
        let actual = aggr.act(&live_updated_2).await.unwrap();
        let expected: Vec<MsgData> = vec![MsgData::Sell];
        assert_eq!(expected, actual)
    }

    #[async_std::test]
    async fn actor_should_emit_nothing_if_live_price_stays_below_average() {
        let mut aggr = SimpleCrossover::new();
        let average_updated = Msg::with_data(MsgData::AveragePriceUpdated(PriceUpdated {
            pair_id: "pair_id",
            datetime: 0,
            price: 1.0,
            ..Default::default()
        }));
        let live_updated_1 = Msg::with_data(MsgData::LivePriceUpdated(PriceUpdated {
            pair_id: "pair_id",
            datetime: SECOND,
            price: 0.7,
            ..Default::default()
        }));
        let live_updated_2 = Msg::with_data(MsgData::LivePriceUpdated(PriceUpdated {
            pair_id: "pair_id",
            datetime: SECOND * 2,
            price: 0.1,
            ..Default::default()
        }));
        aggr.act(&average_updated).await.unwrap();
        aggr.act(&live_updated_1).await.unwrap();
        let actual = aggr.act(&live_updated_2).await.unwrap();
        let expected: Vec<MsgData> = vec![];
        assert_eq!(expected, actual)
    }
}
