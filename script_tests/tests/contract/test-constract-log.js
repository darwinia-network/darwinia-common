// Run the transfer balance firstly.
const expect = require('chai').expect;
const Web3 = require('web3');
const conf = require('../config.js');

const web3 = new Web3(conf.host);
const account = web3.eth.accounts.wallet.add(conf.privKey);
const jsontest = new web3.eth.Contract(conf.abi)
jsontest.options.from = conf.address;
jsontest.options.gas = conf.gas;

describe('Test Contract Log', function () {

   it('Deploy json test contract', async function () {
      const instance = await jsontest.deploy({
         data: conf.bytecode,
         arguments: [],
      }).send();
      jsontest.options.address = instance.options.address;
   }).timeout(10000);

   it('Get default bool value', async function () {
       const data = await jsontest.methods
          .getBool()
          .call();
       expect(data).to.be.false;
   });


   it('Set bool to true', async function () {
      const value = true;
      await jsontest.methods.setBool(value).send();
      const data = await jsontest.methods
        .getBool()
        .call();
      expect(data).to.be.equal(value);
   }).timeout(80000);

   it('Fire event log0', async function () {
      jsontest.methods.fireEventLog0().send();
      jsontest.once('Log0', {
        fromBlock: 0
      }, function(error, event){ 
        expect(event.raw.data).to.be.equal('0x000000000000000000000000000000000000000000000000000000000000002a');
        expect(event.signature).to.be.equal('0x65c9ac8011e286e89d02a269890f41d67ca2cc597b2c76c7c69321ff492be580');
      });
   }).timeout(80000);

   it('Fire event Log0Anonym', async function () {
      jsontest.methods.fireEventLog0Anonym().send();
      jsontest.once('Log0Anonym', {
        fromBlock: 0
      }, function(error, event){ 
        expect(event.raw.data).to.be.equal('0x000000000000000000000000000000000000000000000000000000000000002a');
        expect(event.signature).to.be.null;
      });
   }).timeout(80000);

   it('Fire event Log1', async function () {
      jsontest.methods.fireEventLog1().send();
      jsontest.once('Log1', {
        fromBlock: 0
      }, function(error, event){ 
        console.log(event)
      });
   }).timeout(80000);
});
