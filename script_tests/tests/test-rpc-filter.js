const expect = require('chai').expect;
const assert = require('chai').assert;
const Web3 = require('web3');
const utils = require('./utils');
const conf = require('./config.js');

describe('Test Net API', function () {

    it('should return a list of logs, when asking without defining an address and using toBlock "latest"', async function () {
        const params = [{
            "fromBlock": '0x0',
            "toBlock": 'latest'
        }]
        const filterId = await utils.customRequest('eth_newFilter', params);
        assert.isNumber(filterId);
        const logs = await utils.customRequest('eth_getFilterLogs', [filterId]);
        console.log(logs);
        await utils.customRequest('eth_uninstallFilter', [filterId]);
    });

});

