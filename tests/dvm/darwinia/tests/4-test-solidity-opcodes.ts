import Web3 from "web3";
import { expect, assert } from "chai";
import { config } from "./config";
import { AbiItem } from "web3-utils";
import { bytecode } from "./bytecode";

const web3 = new Web3(config.host);

const account = web3.eth.accounts.wallet.add(config.privKey);
const opcodes = new web3.eth.Contract(bytecode.abi as AbiItem[]);
opcodes.options.from = config.address;
opcodes.options.gas = config.gas;
opcodes.options.gasPrice = "1000000000";

describe("Test Solidity OpCodes", function () {
	it("Should run without errors the majorit of opcodes", async () => {
		const instance = await opcodes
			.deploy({
				// TODO: udpate this data
				data: bytecode.bytecode,
				arguments: [],
			})
			.send({
				from: config.address,
			});
		opcodes.options.address = instance.options.address;
		await opcodes.methods.test().send();
		await opcodes.methods.test_stop().send();
	}).timeout(120000);

	it("Should throw invalid op code", async () => {
		try {
			await opcodes.methods.test_invalid().send();
		} catch (error) {
			expect(error.receipt.status).to.be.false;
		}
	}).timeout(120000);

	it("Should revert", async () => {
		try {
			await opcodes.methods.test_revert().send();
		} catch (error) {
			expect(error.receipt.status).to.be.false;
		}
	}).timeout(120000);
});
