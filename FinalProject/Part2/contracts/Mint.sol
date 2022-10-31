// SPDX-License-Identifier: MIT
pragma solidity >=0.8.0 <0.9.0;

import "@openzeppelin/contracts/access/Ownable.sol";
import "./interfaces/IPriceFeed.sol";
import "./interfaces/IMint.sol";
import "./sAsset.sol";
import "./EUSD.sol";

contract Mint is Ownable, IMint{

    struct Asset {
        address token;
        uint minCollateralRatio;
        address priceFeed;
    }

    struct Position {
        uint idx;
        address owner;
        uint collateralAmount;
        address assetToken;
        uint assetAmount;
        // Added this bool to keep track of deleted entries
        bool deleted;
    }

    mapping(address => Asset) _assetMap;
    uint _currentPositionIndex;
    mapping(uint => Position) _idxPositionMap;
    address public collateralToken;
    

    constructor(address collateral) {
        collateralToken = collateral;
    }

    function registerAsset(address assetToken, uint minCollateralRatio, address priceFeed) external override onlyOwner {
        require(assetToken != address(0), "Invalid assetToken address");
        require(minCollateralRatio >= 1, "minCollateralRatio must be greater than 100%");
        require(_assetMap[assetToken].token == address(0), "Asset was already registered");
        
        _assetMap[assetToken] = Asset(assetToken, minCollateralRatio, priceFeed);
    }

    function getPosition(uint positionIndex) external view returns (address, uint, address, uint) {
        require(positionIndex < _currentPositionIndex, "Invalid index");
        Position storage position = _idxPositionMap[positionIndex];
        return (position.owner, position.collateralAmount, position.assetToken, position.assetAmount);
    }

    function getMintAmount(uint collateralAmount, address assetToken, uint collateralRatio) public view returns (uint) {
        Asset storage asset = _assetMap[assetToken];
        (int relativeAssetPrice, ) = IPriceFeed(asset.priceFeed).getLatestPrice();
        uint8 decimal = sAsset(assetToken).decimals();
        uint mintAmount = collateralAmount * (10 ** uint256(decimal)) / uint(relativeAssetPrice) / collateralRatio ;
        return mintAmount;
    }

    function checkRegistered(address assetToken) public view returns (bool) {
        return _assetMap[assetToken].token == assetToken;
    }

    /* TODO: implement your functions here */

    function openPosition(uint collateralAmount, address assetToken, uint collateralRatio) external override {
        require(assetToken != address(0), "Invalid assetToken address");
        require(checkRegistered(assetToken), "Asset not registered"); 
        require(collateralRatio >= _assetMap[assetToken].minCollateralRatio, "collateralRatio must be greater than minCollateralRatio");

        // Is this needed?
        // EUSD(collateralToken).approve(msg.sender, collateralAmount);
        // Send collateral tokens from msg.sender to the contract address
        EUSD(collateralToken).transferFrom(msg.sender, address(this), collateralAmount);

        uint mintAmount = getMintAmount(collateralAmount, assetToken, collateralRatio);
        // new_pos.idx = _currentPositionIndex;
        // new_pos.owner = msg.sender;
        // new_pos.collateralAmount = collateralAmount; 
        // new_pos.assetToken = assetToken;
        // new_pos.assetAmount = mintAmount; 

        // mint new tokens altogether right? 
        // Is sAsset(assetToken) calling the constructor? wtf is it doing? Why is its one paramater an address?

        // Asset storage asset = _assetMap[assetToken];
        // require(collateralAmount / mintAmount >= asset.minCollateralRatio, "Collateral ratio cannot go below the MCR");

        sAsset(assetToken).mint(msg.sender, mintAmount); 

        // Add to _idxPositionMap
        _idxPositionMap[_currentPositionIndex] = Position(_currentPositionIndex, msg.sender, collateralAmount, assetToken, mintAmount, false);
        _currentPositionIndex += 1;
    }

    function closePosition(uint positionIndex) external override {
        Position storage pos = _idxPositionMap[positionIndex]; 
        require(pos.deleted == false, "Position doesn't exist"); 
        require(pos.owner == msg.sender, "Message sender doesn't own the position"); 

        // Transfer sAsset tokens from msg.sender to contract
        // sAsset(pos.assetToken).transferFrom(msg.sender, address(this), pos.assetAmount);

        // Is this needed?
        // sAsset(pos.assetToken).approve(address(this), pos.assetAmount);
        // Burn sAsset tokens
        // sAsset(pos.assetToken).burn(address(this), pos.assetAmount);

        sAsset(pos.assetToken).burn(msg.sender, pos.assetAmount);

        EUSD(collateralToken).approve(address(this), pos.collateralAmount);
        // Transfer collateral tokens back from contract to msg.sender
        EUSD(collateralToken).transferFrom(address(this), msg.sender, pos.collateralAmount);

        // Remove position from _idxPositionMap
        _idxPositionMap[positionIndex].deleted = true;
        delete _idxPositionMap[positionIndex]; 
    }

    function deposit(uint positionIndex, uint collateralAmount) external override {
        Position storage pos = _idxPositionMap[positionIndex]; 
        require(pos.deleted == false, "Position doesn't exist"); 
        require(pos.owner == msg.sender, "Message sender doesn't own the position"); 

        EUSD(collateralToken).transferFrom(msg.sender, address(this), collateralAmount);
        pos.collateralAmount += collateralAmount;
    }

    function withdraw(uint positionIndex, uint withdrawAmount) external override {
        Position storage pos = _idxPositionMap[positionIndex]; 
        require(pos.deleted == false, "Position doesn't exist"); 
        require(pos.owner == msg.sender, "Message sender doesn't own the position");

        Asset storage asset = _assetMap[pos.assetToken]; 
        require((pos.collateralAmount - withdrawAmount) / pos.assetAmount >= asset.minCollateralRatio, "Collateral ratio cannot go below the MCR");

        EUSD(collateralToken).approve(address(this), withdrawAmount);
        EUSD(collateralToken).transferFrom(address(this), msg.sender, withdrawAmount);
        pos.collateralAmount -= withdrawAmount;
    }

    function mint(uint positionIndex, uint mintAmount) external override {
        Position storage pos = _idxPositionMap[positionIndex]; 
        require(pos.deleted == false, "Position doesn't exist"); 
        require(pos.owner == msg.sender, "Message sender doesn't own the position");
        
        Asset storage asset = _assetMap[pos.assetToken];
        require(pos.collateralAmount / (pos.assetAmount + mintAmount) >= asset.minCollateralRatio, "Collateral ratio cannot go below the MCR");

        sAsset(pos.assetToken).mint(msg.sender, mintAmount); 
        pos.assetAmount += mintAmount;
    }

    function burn(uint positionIndex, uint burnAmount) external override {
        Position storage pos = _idxPositionMap[positionIndex]; 
        require(pos.deleted == false, "Position doesn't exist anymore"); 
        require(pos.owner == msg.sender, "Message sender doesn't own the position");

        sAsset(pos.assetToken).burn(msg.sender, burnAmount); 
        pos.assetAmount -= burnAmount;
    }
    
}