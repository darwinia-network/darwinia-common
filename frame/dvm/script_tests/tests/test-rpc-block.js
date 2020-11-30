const expect = require('chai').expect;
const Web3 = require('web3');

const web3 = new Web3('http://localhost:9933');

describe('Test Block RPC', function () {

	it('The block number should not be zero', async function () {
		expect(await web3.eth.getBlockNumber()).to.not.equal(0);
	});

	it('Should return the genesis block', async function () {
		const block = await web3.eth.getBlock(0);
		expect(block).to.include({
			author: "0x0000000000000000000000000000000000000000",
			difficulty: "0",
			// extraData: "0x",
			gasLimit: 0,
			gasUsed: 0,
			//hash: "0x14fe6f7c93597f79b901f8b5d7a84277a90915b8d355959b587e18de34f1dc17",
			logsBloom:
				"0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
			miner: "0x0000000000000000000000000000000000000000",
			number: 0,
			//parentHash: "0x2cc74f91423ba20e9bb0b2c7d8924eacd14bc98aa1daad078f8844e529221cde",
			receiptsRoot: "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
			// size: 501,
			stateRoot: "0x0000000000000000000000000000000000000000000000000000000000000000",
			timestamp: 0,
			totalDifficulty: null,
			//transactions: [],
			transactionsRoot: "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
			//uncles: []
		});

		expect(block.transactions).to.be.a("array").empty;
		expect(block.uncles).to.be.a("array").empty;
		expect(block.sealFields).to.eql([
			"0x0000000000000000000000000000000000000000000000000000000000000000",
			"0x0000000000000000",
		]);
		expect(block.hash).to.be.a("string").lengthOf(66);
		expect(block.parentHash).to.be.a("string").lengthOf(66);
		expect(block.timestamp).to.be.a("number");
	});

	it("get block by hash", async function () {
		const latest_block = await web3.eth.getBlock("latest");
		const block = await web3.eth.getBlock(latest_block.hash);
		expect(block.hash).to.be.eq(latest_block.hash);
	});

	it("get block by number", async function () {
		const block = await web3.eth.getBlock(3);
		expect(block.number).to.equal(3);
	});

	it("should include previous block hash as parent", async function () {
		const block = await web3.eth.getBlock("latest");

		// previous block
		const previous_block_number = block.number - 1;
		const previous_block = await web3.eth.getBlock(previous_block_number);

		expect(block.hash).to.not.equal(previous_block.hash);
		expect(block.parentHash).to.equal(previous_block.hash);
	});

	it("should have valid timestamp after block production", async function () {
		const block = await web3.eth.getBlock("latest");

		// previous block
		const previous_block_number = block.number - 1;
		const previous_block = await web3.eth.getBlock(previous_block_number);

		expect(block.timestamp - previous_block.timestamp).to.be.eq(6);
	});
});
