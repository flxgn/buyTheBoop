use crate::messaging::message::{Msg, MsgData, Price};
use crate::messaging::processor::Actor;
use anyhow::Result;
use async_trait::async_trait;

#[derive(Debug, PartialEq, Clone, Default)]
pub struct SimpleCrossover {
    offset: f64,
    latest_average: Option<Price>,
    latest_live: Option<Price>,
}

impl SimpleCrossover {
    pub fn new(offset: f64) -> Self {
        SimpleCrossover {
            offset,
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
                    .latest_average
                    .map(|avg| {
                        if e.price > avg * (1.0 + self.offset)
                            && (self.latest_live.is_none() || self.latest_live < Some(avg * (1.0 + self.offset)))
                        {
                            vec![MsgData::Buy]
                        } else if e.price < avg * (1.0 - self.offset)
                            && (self.latest_live.is_none() || self.latest_live > Some(avg * (1.0 - self.offset)))
                        {
                            vec![MsgData::Sell]
                        } else {
                            vec![]
                        }
                    })
                    .unwrap_or(vec![]);
                if self.latest_average.is_some() {
                    self.latest_live = Some(e.price);
                }
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

    const SECOND: u128 = 1_000;

    #[async_std::test]
    async fn actor_should_emit_nothing_if_only_average_price_updated() {
        let mut aggr = SimpleCrossover::new(0.0);
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
        let mut aggr = SimpleCrossover::new(0.0);
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
        let mut aggr = SimpleCrossover::new(0.0);
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
        let mut aggr = SimpleCrossover::new(0.0);
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
    async fn actor_should_emit_buy_msg_if_live_price_starts_above_average() {
        let mut aggr = SimpleCrossover::new(0.0);
        let average_updated = Msg::with_data(MsgData::AveragePriceUpdated(PriceUpdated {
            pair_id: "pair_id",
            datetime: 0,
            price: 1.0,
            ..Default::default()
        }));
        let live_updated = Msg::with_data(MsgData::LivePriceUpdated(PriceUpdated {
            pair_id: "pair_id",
            datetime: SECOND,
            price: 1.1,
            ..Default::default()
        }));
        aggr.act(&average_updated).await.unwrap();
        let actual = aggr.act(&live_updated).await.unwrap();
        let expected: Vec<MsgData> = vec![MsgData::Buy];
        assert_eq!(expected, actual)
    }

    #[async_std::test]
    async fn actor_should_emit_buy_msg_if_live_price_starts_above_average_with_prior_already_above()
    {
        let mut aggr = SimpleCrossover::new(0.0);
        let live_updated_1 = Msg::with_data(MsgData::LivePriceUpdated(PriceUpdated {
            pair_id: "pair_id",
            datetime: SECOND,
            price: 1.1,
            ..Default::default()
        }));
        let average_updated = Msg::with_data(MsgData::AveragePriceUpdated(PriceUpdated {
            pair_id: "pair_id",
            datetime: 0,
            price: 1.0,
            ..Default::default()
        }));
        let live_updated_2 = Msg::with_data(MsgData::LivePriceUpdated(PriceUpdated {
            pair_id: "pair_id",
            datetime: SECOND,
            price: 1.2,
            ..Default::default()
        }));
        aggr.act(&live_updated_1).await.unwrap();
        aggr.act(&average_updated).await.unwrap();
        let actual = aggr.act(&live_updated_2).await.unwrap();
        let expected: Vec<MsgData> = vec![MsgData::Buy];
        assert_eq!(expected, actual)
    }

    #[async_std::test]
    async fn actor_should_emit_no_buy_if_average_price_update_after_live() {
        let mut aggr = SimpleCrossover::new(0.0);
        let live_updated = Msg::with_data(MsgData::LivePriceUpdated(PriceUpdated {
            pair_id: "pair_id",
            datetime: SECOND,
            price: 1.1,
            ..Default::default()
        }));
        let average_updated = Msg::with_data(MsgData::AveragePriceUpdated(PriceUpdated {
            pair_id: "pair_id",
            datetime: 0,
            price: 1.0,
            ..Default::default()
        }));

        aggr.act(&live_updated).await.unwrap();
        let actual = aggr.act(&average_updated).await.unwrap();
        let expected: Vec<MsgData> = vec![];
        assert_eq!(expected, actual)
    }

    #[async_std::test]
    async fn actor_should_emit_sell_msg_if_live_price_crosses_average_downwards() {
        let mut aggr = SimpleCrossover::new(0.0);
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
        let mut aggr = SimpleCrossover::new(0.0);
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

    #[async_std::test]
    async fn actor_should_emit_sell_msg_if_live_price_starts_below_average() {
        let mut aggr = SimpleCrossover::new(0.0);
        let average_updated = Msg::with_data(MsgData::AveragePriceUpdated(PriceUpdated {
            pair_id: "pair_id",
            datetime: 0,
            price: 1.0,
            ..Default::default()
        }));
        let live_updated = Msg::with_data(MsgData::LivePriceUpdated(PriceUpdated {
            pair_id: "pair_id",
            datetime: SECOND,
            price: 0.9,
            ..Default::default()
        }));
        aggr.act(&average_updated).await.unwrap();
        let actual = aggr.act(&live_updated).await.unwrap();
        let expected: Vec<MsgData> = vec![MsgData::Sell];
        assert_eq!(expected, actual)
    }

    #[async_std::test]
    async fn actor_should_emit_sell_msg_if_live_price_starts_below_average_with_prior_already_below(
    ) {
        let mut aggr = SimpleCrossover::new(0.0);
        let live_updated_1 = Msg::with_data(MsgData::LivePriceUpdated(PriceUpdated {
            pair_id: "pair_id",
            datetime: SECOND,
            price: 0.9,
            ..Default::default()
        }));
        let average_updated = Msg::with_data(MsgData::AveragePriceUpdated(PriceUpdated {
            pair_id: "pair_id",
            datetime: 0,
            price: 1.0,
            ..Default::default()
        }));
        let live_updated_2 = Msg::with_data(MsgData::LivePriceUpdated(PriceUpdated {
            pair_id: "pair_id",
            datetime: SECOND,
            price: 0.8,
            ..Default::default()
        }));
        aggr.act(&live_updated_1).await.unwrap();
        aggr.act(&average_updated).await.unwrap();
        let actual = aggr.act(&live_updated_2).await.unwrap();
        let expected: Vec<MsgData> = vec![MsgData::Sell];
        assert_eq!(expected, actual)
    }

    #[async_std::test]
    async fn actor_should_emit_no_sell_if_average_price_update_after_live() {
        let mut aggr = SimpleCrossover::new(0.0);
        let live_updated = Msg::with_data(MsgData::LivePriceUpdated(PriceUpdated {
            pair_id: "pair_id",
            datetime: SECOND,
            price: 0.8,
            ..Default::default()
        }));
        let average_updated = Msg::with_data(MsgData::AveragePriceUpdated(PriceUpdated {
            pair_id: "pair_id",
            datetime: 0,
            price: 1.0,
            ..Default::default()
        }));

        aggr.act(&live_updated).await.unwrap();
        let actual = aggr.act(&average_updated).await.unwrap();
        let expected: Vec<MsgData> = vec![];
        assert_eq!(expected, actual)
    }

    #[async_std::test]
    async fn actor_should_not_emit_buy_msg_if_live_price_starts_above_average_but_below_offset() {
        let mut aggr = SimpleCrossover::new(0.1);
        let average_updated = Msg::with_data(MsgData::AveragePriceUpdated(PriceUpdated {
            pair_id: "pair_id",
            datetime: 0,
            price: 1.0,
            ..Default::default()
        }));
        let live_updated = Msg::with_data(MsgData::LivePriceUpdated(PriceUpdated {
            pair_id: "pair_id",
            datetime: SECOND,
            price: 1.04,
            ..Default::default()
        }));
        aggr.act(&average_updated).await.unwrap();
        let actual = aggr.act(&live_updated).await.unwrap();
        let expected: Vec<MsgData> = vec![];
        assert_eq!(expected, actual)
    }

    #[async_std::test]
    async fn actor_should_emit_buy_msg_if_live_price_crosses_average_upwards_with_offset() {
        let mut aggr = SimpleCrossover::new(0.3);
        let average_updated = Msg::with_data(MsgData::AveragePriceUpdated(PriceUpdated {
            pair_id: "pair_id",
            datetime: 0,
            price: 1.0,
            ..Default::default()
        }));
        let live_updated_1 = Msg::with_data(MsgData::LivePriceUpdated(PriceUpdated {
            pair_id: "pair_id",
            datetime: SECOND,
            price: 1.2,
            ..Default::default()
        }));
        let live_updated_2 = Msg::with_data(MsgData::LivePriceUpdated(PriceUpdated {
            pair_id: "pair_id",
            datetime: SECOND * 2,
            price: 1.5,
            ..Default::default()
        }));
        aggr.act(&average_updated).await.unwrap();
        aggr.act(&live_updated_1).await.unwrap();
        let actual = aggr.act(&live_updated_2).await.unwrap();
        let expected: Vec<MsgData> = vec![MsgData::Buy];
        assert_eq!(expected, actual)
    }

    #[async_std::test]
    async fn actor_should_emit_buy_msg_if_live_price_starts_above_average_with_offset() {
        let mut aggr = SimpleCrossover::new(0.3);
        let average_updated = Msg::with_data(MsgData::AveragePriceUpdated(PriceUpdated {
            pair_id: "pair_id",
            datetime: 0,
            price: 1.0,
            ..Default::default()
        }));
        let live_updated = Msg::with_data(MsgData::LivePriceUpdated(PriceUpdated {
            pair_id: "pair_id",
            datetime: SECOND,
            price: 1.4,
            ..Default::default()
        }));
        aggr.act(&average_updated).await.unwrap();
        let actual = aggr.act(&live_updated).await.unwrap();
        let expected: Vec<MsgData> = vec![MsgData::Buy];
        assert_eq!(expected, actual)
    }

    #[async_std::test]
    async fn actor_should_not_emit_sell_msg_if_live_price_starts_below_average_but_above_offset() {
        let mut aggr = SimpleCrossover::new(0.1);
        let average_updated = Msg::with_data(MsgData::AveragePriceUpdated(PriceUpdated {
            pair_id: "pair_id",
            datetime: 0,
            price: 1.0,
            ..Default::default()
        }));
        let live_updated = Msg::with_data(MsgData::LivePriceUpdated(PriceUpdated {
            pair_id: "pair_id",
            datetime: SECOND,
            price: 0.95,
            ..Default::default()
        }));
        aggr.act(&average_updated).await.unwrap();
        let actual = aggr.act(&live_updated).await.unwrap();
        let expected: Vec<MsgData> = vec![];
        assert_eq!(expected, actual)
    }

    #[async_std::test]
    async fn actor_should_emit_sell_msg_if_live_price_crosses_average_downwards_with_offset() {
        let mut aggr = SimpleCrossover::new(0.3);
        let average_updated = Msg::with_data(MsgData::AveragePriceUpdated(PriceUpdated {
            pair_id: "pair_id",
            datetime: 0,
            price: 1.0,
            ..Default::default()
        }));
        let live_updated_1 = Msg::with_data(MsgData::LivePriceUpdated(PriceUpdated {
            pair_id: "pair_id",
            datetime: SECOND,
            price: 0.9,
            ..Default::default()
        }));
        let live_updated_2 = Msg::with_data(MsgData::LivePriceUpdated(PriceUpdated {
            pair_id: "pair_id",
            datetime: SECOND * 2,
            price: 0.5,
            ..Default::default()
        }));
        aggr.act(&average_updated).await.unwrap();
        aggr.act(&live_updated_1).await.unwrap();
        let actual = aggr.act(&live_updated_2).await.unwrap();
        let expected: Vec<MsgData> = vec![MsgData::Sell];
        assert_eq!(expected, actual)
    }

    #[async_std::test]
    async fn actor_should_emit_sell_msg_if_live_price_starts_below_average_with_offset() {
        let mut aggr = SimpleCrossover::new(0.3);
        let average_updated = Msg::with_data(MsgData::AveragePriceUpdated(PriceUpdated {
            pair_id: "pair_id",
            datetime: 0,
            price: 1.0,
            ..Default::default()
        }));
        let live_updated = Msg::with_data(MsgData::LivePriceUpdated(PriceUpdated {
            pair_id: "pair_id",
            datetime: SECOND,
            price: 0.6,
            ..Default::default()
        }));
        aggr.act(&average_updated).await.unwrap();
        let actual = aggr.act(&live_updated).await.unwrap();
        let expected: Vec<MsgData> = vec![MsgData::Sell];
        assert_eq!(expected, actual)
    }

}
