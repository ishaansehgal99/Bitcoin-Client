// SPDX-License-Identifier: MIT
pragma solidity >=0.8.0 <0.9.0;
import "@openzeppelin/contracts/access/Ownable.sol";
import "./interfaces/ISwap.sol";
import "./sAsset.sol";

contract Swap is Ownable, ISwap {

    address token0;
    address token1;
    uint reserve0;
    uint reserve1;
    mapping (address => uint) shares;
    uint public totalShares;

    constructor(address addr0, address addr1) {
        token0 = addr0;
        token1 = addr1;
    }

    function init(uint token0Amount, uint token1Amount) external override onlyOwner {
        require(reserve0 == 0 && reserve1 == 0, "init - already has liquidity");
        require(token0Amount > 0 && token1Amount > 0, "init - both tokens are needed");
        
        require(sAsset(token0).transferFrom(msg.sender, address(this), token0Amount));
        require(sAsset(token1).transferFrom(msg.sender, address(this), token1Amount));
        reserve0 = token0Amount;
        reserve1 = token1Amount;
        totalShares = sqrt(token0Amount * token1Amount);
        shares[msg.sender] = totalShares;
    }

    // https://github.com/Uniswap/v2-core/blob/v1.0.1/contracts/libraries/Math.sol
    function sqrt(uint y) internal pure returns (uint z) {
        if (y > 3) {
            z = y;
            uint x = y / 2 + 1;
            while (x < z) {
                z = x;
                x = (y / x + x) / 2;
            }
        } else if (y != 0) {
            z = 1;
        }
    }

    function getReserves() external view returns (uint, uint) {
        return (reserve0, reserve1);
    }

    function getTokens() external view returns (address, address) {
        return (token0, token1);
    }

    function getShares(address LP) external view returns (uint) {
        return shares[LP];
    }

    /* TODO: implement your functions here */

    function addLiquidity(uint token0Amount) external override {
        // Ensure token1Amount / reserve1 == token0Amount / reserve0
        // Init
        // token0 / reserve0: 10 / 10 and token1 / reserve1: 5 / 5  
        // Add Liquidity
        // token1 amoint: 1 / 2 == token0 amoutn 1
        // 1 / 11 == .5 / 5.5
        require (reserve0 > 0 && reserve1 > 0);
        require(sAsset(token0).transferFrom(msg.sender, address(this), token0Amount));

        uint token1Amount = (reserve1 * token0Amount) / reserve0;
        require(sAsset(token1).transferFrom(msg.sender, address(this), token1Amount));

        uint new_shares = (totalShares * token0Amount) / reserve0;
        reserve0 += token0Amount;
        reserve1 += token1Amount;
        shares[msg.sender] += new_shares; 
        totalShares += new_shares;
    }

    function removeLiquidity(uint withdrawShares) external override {
        require(withdrawShares <= shares[msg.sender], "Insufficient shares.");
        uint amount0 = (reserve0 * withdrawShares) / totalShares;
        uint amount1 = (reserve1 * withdrawShares) / totalShares;

        require(sAsset(token0).approve(address(this), amount0));
        require(sAsset(token0).transferFrom(address(this), msg.sender, amount0));

        require(sAsset(token1).approve(address(this), amount1));
        require(sAsset(token1).transferFrom(address(this), msg.sender, amount1));
        reserve0 -= amount0;
        reserve1 -= amount1;
        shares[msg.sender] -= withdrawShares;
        totalShares -= withdrawShares;
    }
    
    function token0To1(uint token0Amount) external override {  
        if (token0Amount == 0) {
            return; 
        }

        uint token0_to_exchange = (token0Amount * 997) / 1000;
        // newreserce = (1000000, 0)

        // token1_to_return = 100 - 100 * 100 / (100 + 48,500) = 99.79 
    
        // reserve 100, 100
        // og 100 * 100
        // transfer 50000
        // token1_to_return = 100 - 100 * 100 / (100 + 9999) = 99.0098029508

        // invariant = (100 + 9999) * (100-99.0098029508) = 10,000
        // new reserve: 50100, .2
        
        uint token1_to_return = reserve1 - reserve1 * reserve0 / (reserve0 + token0_to_exchange);
        require(token1_to_return > 0, "Transaction invalid");

        uint invariant = (reserve0 + token0_to_exchange) * (reserve1 - token1_to_return);
        require(invariant > 0, "Transaction invalid");

        // uint token1_to_return_og = reserve1 - reserve1 * reserve0 / (reserve0 + token0Amount);
        // require(reserve0 * reserve1 == (reserve0 + token0Amount) * (reserve1 * token1_to_return_og));

        // uint payoutToken0Amount = token0Amount - token0_to_exchange;
        // uint new_shares = (totalShares * payoutToken0Amount) / reserve0;
        // shares[msg.sender] += new_shares; 
        // totalShares += new_shares;
        
        reserve0 += token0Amount;
        reserve1 -= token1_to_return;

        require(sAsset(token0).transferFrom(msg.sender, address(this), token0Amount));

        require(sAsset(token1).approve(address(this), token1_to_return));
        require(sAsset(token1).transferFrom(address(this), msg.sender, token1_to_return));
    }

    function token1To0(uint token1Amount) external override {
        if (token1Amount == 0) {
            return; 
        }
        uint token1_to_exchange = (token1Amount * 997) / 1000; 
        uint token0_to_return = reserve0 - (reserve0 * reserve1) / (reserve1 + token1_to_exchange);
        require(token0_to_return > 0, "Transaction invalid");

        uint invariant = (reserve1 + token1_to_exchange) * (reserve0 - token0_to_return);
        require(invariant > 0, "Transaction invalid");

        // uint payoutToken0Amount = token0Amount - token0_to_exchange;
        // uint new_shares = (totalShares * payoutToken0Amount) / reserve0;
        // shares[msg.sender] += new_shares; 
        // totalShares += new_shares;

        reserve0 -= token0_to_return;
        reserve1 += token1Amount;

        require(sAsset(token1).transferFrom(msg.sender, address(this), token1Amount));

        require(sAsset(token0).approve(address(this), token0_to_return));
        require(sAsset(token0).transferFrom(address(this), msg.sender, token0_to_return));
    }

}