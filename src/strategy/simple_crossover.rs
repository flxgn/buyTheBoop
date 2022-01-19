use crate::messaging::message::{Msg, Price};
use crate::messaging::processor::Aggregator;
use async_trait::async_trait;
use anyhow::Result;

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
impl<'a> Aggregator<'a> for SimpleCrossover {
    async fn aggregate(&mut self, msg: &Msg<'a>) -> Result<Vec<Msg<'a>>> {
        let res = match msg {
            Msg::LivePriceUpdated(e) => {
                let result = self
                    .latest_live
                    .and_then(|live| {
                        self.latest_average.map(|avg| {
                            if e.price > avg && live < avg {
                                vec![Msg::Buy]
                            } else if e.price < avg && live > avg {
                                vec![Msg::Sell]
                            } else {
                                vec![]
                            }
                        })
                    })
                    .unwrap_or(vec![]);
                self.latest_live = Some(e.price);
                result
            }
            Msg::AveragePriceUpdated(e) => {
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

    const SECOND: i64 = 1_000;

    #[async_std::test]
    async fn aggr_should_emit_nothing_if_only_average_price_updated() {
        let mut aggr = SimpleCrossover::new();
        let msg = Msg::AveragePriceUpdated(PriceUpdated {
            pair_id: "pair_id",
            datetime: 0,
            price: 1.0,
            ..Default::default()
        });
        let actual = aggr.aggregate(&msg).await.unwrap();
        let expected: Vec<Msg> = vec![];
        assert_eq!(expected, actual)
    }

    #[async_std::test]
    async fn aggr_should_emit_nothing_if_only_live_price_updated() {
        let mut aggr = SimpleCrossover::new();
        let msg = Msg::LivePriceUpdated(PriceUpdated {
            pair_id: "pair_id",
            datetime: 0,
            price: 1.0,
            ..Default::default()
        });
        let actual = aggr.aggregate(&msg).await.unwrap();
        let expected: Vec<Msg> = vec![];
        assert_eq!(expected, actual)
    }

    #[async_std::test]
    async fn aggr_should_emit_buy_msg_if_live_price_crosses_average_upwards() {
        let mut aggr = SimpleCrossover::new();
        let average_updated = Msg::AveragePriceUpdated(PriceUpdated {
            pair_id: "pair_id",
            datetime: 0,
            price: 1.0,
            ..Default::default()
        });
        let live_updated_1 = Msg::LivePriceUpdated(PriceUpdated {
            pair_id: "pair_id",
            datetime: SECOND,
            price: 0.5,
            ..Default::default()
        });
        let live_updated_2 = Msg::LivePriceUpdated(PriceUpdated {
            pair_id: "pair_id",
            datetime: SECOND * 2,
            price: 1.1,
            ..Default::default()
        });
        aggr.aggregate(&average_updated).await.unwrap();
        aggr.aggregate(&live_updated_1).await.unwrap();
        let actual = aggr.aggregate(&live_updated_2).await.unwrap();
        let expected: Vec<Msg> = vec![Msg::Buy];
        assert_eq!(expected, actual)
    }

    #[async_std::test]
    async fn aggr_should_emit_nothing_if_live_price_stays_above_average() {
        let mut aggr = SimpleCrossover::new();
        let average_updated = Msg::AveragePriceUpdated(PriceUpdated {
            pair_id: "pair_id",
            datetime: 0,
            price: 1.0,
            ..Default::default()
        });
        let live_updated_1 = Msg::LivePriceUpdated(PriceUpdated {
            pair_id: "pair_id",
            datetime: SECOND,
            price: 1.1,
            ..Default::default()
        });
        let live_updated_2 = Msg::LivePriceUpdated(PriceUpdated {
            pair_id: "pair_id",
            datetime: SECOND * 2,
            price: 1.2,
            ..Default::default()
        });
        aggr.aggregate(&average_updated).await.unwrap();
        aggr.aggregate(&live_updated_1).await.unwrap();
        let actual = aggr.aggregate(&live_updated_2).await.unwrap();
        let expected: Vec<Msg> = vec![];
        assert_eq!(expected, actual)
    }

    #[async_std::test]
    async fn aggr_should_emit_sell_msg_if_live_price_crosses_average_downwards() {
        let mut aggr = SimpleCrossover::new();
        let average_updated = Msg::AveragePriceUpdated(PriceUpdated {
            pair_id: "pair_id",
            datetime: 0,
            price: 1.0,
            ..Default::default()
        });
        let live_updated_1 = Msg::LivePriceUpdated(PriceUpdated {
            pair_id: "pair_id",
            datetime: SECOND,
            price: 1.1,
            ..Default::default()
        });
        let live_updated_2 = Msg::LivePriceUpdated(PriceUpdated {
            pair_id: "pair_id",
            datetime: SECOND * 2,
            price: 0.9,
            ..Default::default()
        });
        aggr.aggregate(&average_updated).await.unwrap();
        aggr.aggregate(&live_updated_1).await.unwrap();
        let actual = aggr.aggregate(&live_updated_2).await.unwrap();
        let expected: Vec<Msg> = vec![Msg::Sell];
        assert_eq!(expected, actual)
    }

    #[async_std::test]
    async fn aggr_should_emit_nothing_if_live_price_stays_below_average() {
        let mut aggr = SimpleCrossover::new();
        let average_updated = Msg::AveragePriceUpdated(PriceUpdated {
            pair_id: "pair_id",
            datetime: 0,
            price: 1.0,
            ..Default::default()
        });
        let live_updated_1 = Msg::LivePriceUpdated(PriceUpdated {
            pair_id: "pair_id",
            datetime: SECOND,
            price: 0.7,
            ..Default::default()
        });
        let live_updated_2 = Msg::LivePriceUpdated(PriceUpdated {
            pair_id: "pair_id",
            datetime: SECOND * 2,
            price: 0.1,
            ..Default::default()
        });
        aggr.aggregate(&average_updated).await.unwrap();
        aggr.aggregate(&live_updated_1).await.unwrap();
        let actual = aggr.aggregate(&live_updated_2).await.unwrap();
        let expected: Vec<Msg> = vec![];
        assert_eq!(expected, actual)
    }
}
