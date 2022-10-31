// SPDX-License-Identifier: MIT
pragma solidity >=0.8.0 <0.9.0;

import "@chainlink/contracts/src/v0.8/interfaces/AggregatorV3Interface.sol";
import /*"/interfaces/*/"IPriceFeed.sol";

contract PriceFeed is IPriceFeed {
    /* TODO: implement your functions here */
    AggregatorV3Interface internal priceFeed;

    /*
    * Network: Kovan
    * Aggregator: ETH/USD
    * Address: 0x1Be8e7BB187c83275eB20905a4029Ca152F45872
    */
    constructor() {
        priceFeed = AggregatorV3Interface(0x1Be8e7BB187c83275eB20905a4029Ca152F45872);
    }

    /*
    * Return latest price
    */
    function getLatestPrice() public view override returns (int, uint) {
        (,int price,,
        uint lastUpdatedTime,) = priceFeed.latestRoundData();

        return (price, lastUpdatedTime);
    }
}