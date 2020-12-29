const assert = require('chai').assert;
const Web3 = require('web3');
const BigNumber = require('bignumber.js')
const conf = require('./config.js');
const web3 = new Web3(conf.host);

function customRequest(method, params) {
    return new Promise((resolve, reject) => {
        web3.currentProvider.send({
            method: method,
            params: params,
            jsonrpc: "2.0",
            id: conf.rpcMessageId++
        }, function (error, result) {
            if (error) {
                reject(
                    `Failed to send custom request (${method} (${params.join(",")})): ${
                        error.message || error.toString()
                    }`
                );
            }
            resolve(result)
        })
    })
}

function fromDecimal(number){
    return '0x' + new BigNumber((number).toString(10),10).toString(16);
}

function logTest(result, logInfo){
    assert.isNumber(+result.logIndex, 'logIndex should be a number');
    assert.strictEqual(+result.transactionIndex, logInfo.txIndex, 'transactionIndex should be ' + logInfo.txIndex);
    assert.match(result.transactionHash, /^0x/, 'transactionHash should start with 0x');
    assert.isAbove(result.transactionHash.length, 19, 'transactionHash should be not just "0x"');
    
    // if there was a fork
    if(result.blockHash !== '0x'+ logInfo.block.blockHeader.hash) {
        assert.match(result.blockHash,  /^0x/, 'log blockHash should start with 0x');
        assert.strictEqual(result.blockHash.length,  66, 'log blockHash should be 32bytes');
        assert.isNumber(+result.blockNumber, 'log block number should be a number');

    } else {
        assert.strictEqual(result.blockHash, '0x'+ logInfo.block.blockHeader.hash, 'log blockHash should be 0x' + logInfo.block.blockHeader.hash);
        assert.strictEqual(+result.blockNumber, +logInfo.block.blockHeader.number, 'log block number should be ' + (+logInfo.block.blockHeader.number));
    }
    assert.strictEqual(result.address, '0x'+ logInfo.tx.to, 'log address should 0x'+ logInfo.tx.to);
    assert.isArray(result.topics);
    assert.match(result.data, /^0x/, 'log data should start with 0x');
    if(logInfo.block.reverted && !_.isUndefined(result.polarity))
        assert.equal(!result.polarity, !!logInfo.block.reverted);

    if(!logInfo.anonymous) {
        assert.include(config.compiledTestContract, result.topics[0].replace('0x',''), 'the topic signature should be in the compiled code');
        // and then remove the signature from the topics
        result.topics.shift();
    }

    // test non-indexed params
    var data = (result.data.length <= 66) ? [result.data] : [result.data.slice(0,66), '0x'+ result.data.slice(66)];
    _.each(logInfo.args, function(arg, index){
        if(arg > 0)
            assert.strictEqual(+data[index], arg, 'log data should be a positive number');
        else
            assert.strictEqual(new BigNumber(data[index], 16).minus(new BigNumber('ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff', 16)).minus(1).toNumber(), arg, 'log data should be a negative number');
    });

    // test index args
    _.each(logInfo.indexArgs, function(arg, index){
        if(arg === true) {
            assert.strictEqual(Boolean(result.topics[index]), arg, 'should be TRUE');
        }
        else if(arg === 'msg.sender') {
            assert.isObject(_.find(config.testBlocks.postState, function(value, key){ return key === result.topics[index].slice(26); }), 'should be a existing address in the "postState"');
        } else {
            assert.strictEqual(result.topics[index], arg, 'log topic should match '+ arg);
        }
    });
};

module.exports = {customRequest, fromDecimal, logTest};
