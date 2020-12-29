const expect = require('chai').expect;
const Web3 = require('web3');
const utils = require('./utils');


describe('Test Net API', function () {

    it.skip("should get current network ID", async function () {
        var method = 'eth_getFilterLogs',
        var params = [{
            "fromBlock": utils.fromDecimal(log.block.blockHeader.number),
            "toBlock": utils.fromDecimal(log.block.blockHeader.number)
        }]
        utils.customRequest('', )
        expect(await web3.eth.net.getId()).to.be.equal("43");
    });

    it("should check if the node is listening for peer", async function () {
        expect(await web3.eth.net.isListening()).to.be.equal(true);
    });

    it("should get the number of peers connected to", async function () {
        expect(await web3.eth.net.getPeerCount()).to.be.equal(0);
    });
});

