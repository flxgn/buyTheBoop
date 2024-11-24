# Buy the Boop
Toy project which simulates a crypto exchange and trading strategies applied over time.

## Try it out
You need rust installed on your system:
```
cargo run
```
This will create a `result.svg` file which shows a graph about how the trading strategy performed. 

The lower graph shows the value of the account if you would have just bought and hold (blue line). As well as the value of the assets which were traded with the bot (red line).

The upper graph shows the price of the crypto coin (blue line) and indicates at which points a buy or sell was executed (green and red circles). Additionally, it shows the sliding average and the applied offset at which trades are happening (black lines).

## Strategy
The current strategy aims to buy coin when the current price crosses the average upwards and sells coin when the current price crosses the average downwards.
