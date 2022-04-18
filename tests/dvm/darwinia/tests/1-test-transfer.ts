import { expect } from "chai";
import Web3 from "web3";
import { config } from "./config";

const web3 = new Web3(config.host);

const addressWithdrawPrecompile = "0x0000000000000000000000000000000000000015";
const addressFrom = config.address;
const addressTo = "0xAa01a1bEF0557fa9625581a293F3AA7770192632"; // 2qSbd2umtD4KmV2X7kfttbP8HH4tzL5iMKETbjY2vYXMHHQs
const addressTo2 = "0x44b21a4e1c4a510237c577c936fba2d6153d2fe2"; // 5ELRpquT7C3mWtjepSRH3V2At1xp7MA7mb4uuVNsLDiAWZju

describe("Test Transfer Balance", function () {
	it("Check balance before transfer", async function () {
		const balanceFrom = web3.utils.fromWei(await web3.eth.getBalance(addressFrom), "ether");
		const balanceTo = await web3.utils.fromWei(await web3.eth.getBalance(addressTo), "ether");

		expect(balanceFrom).to.be.equal("123456.78900000000000009");
		expect(balanceTo).to.be.equal("0");
		expect(await web3.eth.getTransactionCount(addressFrom, "latest")).to.eq(0);
		expect(await web3.eth.getTransactionCount(addressFrom, "earliest")).to.eq(0);
	});

	it("Transfer 10 ether", async function () {
		const createTransaction = await web3.eth.accounts.signTransaction(
			{
				from: addressFrom,
				to: addressTo,
				value: web3.utils.toWei("10", "ether"),
				gas: config.gas,
			},
			config.privKey
		);

		const createReceipt = await web3.eth.sendSignedTransaction(
			createTransaction.rawTransaction
		);
	}).timeout(40000);

	it("Check balance after transfer 10 ether", async function () {
		const balanceFrom = web3.utils.fromWei(await web3.eth.getBalance(addressFrom), "ether");
		const balanceTo = await web3.utils.fromWei(await web3.eth.getBalance(addressTo), "ether");

		expect(balanceFrom).to.be.equal("123446.78897900000000009");
		expect(balanceTo).to.be.equal("10");
		expect(await web3.eth.getTransactionCount(addressFrom, "latest")).to.eq(1);
	});

	it("Transfer 100 wei", async function () {
		const createTransaction = await web3.eth.accounts.signTransaction(
			{
				from: addressFrom,
				to: addressTo2,
				value: web3.utils.toWei("100", "wei"),
				gas: config.gas,
			},
			config.privKey
		);
		const createReceipt = await web3.eth.sendSignedTransaction(
			createTransaction.rawTransaction
		);
	}).timeout(40000);

	it("Check balance after transfer 100 wei", async function () {
		const balanceFrom = web3.utils.fromWei(await web3.eth.getBalance(addressFrom), "ether");
		const balanceTo = await web3.utils.fromWei(await web3.eth.getBalance(addressTo2), "ether");

		expect(balanceFrom).to.be.equal("123446.78895799999999999");
		expect(balanceTo).to.be.equal("0.0000000000000001");
		expect(await web3.eth.getTransactionCount(addressFrom, "latest")).to.eq(2);
	});

	it("Transfer 50 ether", async function () {
		const createTransaction = await web3.eth.accounts.signTransaction(
			{
				from: addressFrom,
				to: addressTo,
				value: web3.utils.toWei("50", "ether"),
				gas: config.gas,
			},
			config.privKey
		);

		const createReceipt = await web3.eth.sendSignedTransaction(
			createTransaction.rawTransaction
		);
	}).timeout(40000);

	it("Check balance after transfer 50 ether", async function () {
		const balanceFrom = web3.utils.fromWei(await web3.eth.getBalance(addressFrom), "ether");
		const balanceTo = await web3.utils.fromWei(await web3.eth.getBalance(addressTo), "ether");

		expect(balanceFrom).to.be.equal("123396.78893699999999999");
		expect(balanceTo).to.be.equal("60");
		expect(await web3.eth.getTransactionCount(addressFrom, "latest")).to.eq(3);
	});

	it("Transfer self", async function () {
		const createTransaction = await web3.eth.accounts.signTransaction(
			{
				from: addressFrom,
				to: addressFrom,
				value: web3.utils.toWei("30", "ether"),
				gas: config.gas,
			},
			config.privKey
		);

		const createReceipt = await web3.eth.sendSignedTransaction(
			createTransaction.rawTransaction
		);
	}).timeout(40000);

	it("Check balance after transfer self", async function () {
		const balanceFrom = web3.utils.fromWei(await web3.eth.getBalance(addressFrom), "ether");
		expect(balanceFrom).to.be.equal("123396.78891599999999999");
		expect(await web3.eth.getTransactionCount(addressFrom, "latest")).to.eq(4);
	});
});
