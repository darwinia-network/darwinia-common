const expect = require("chai").expect;
const Web3 = require("web3");

const web3 = new Web3("http://localhost:9933");

const addressWithdrawPrecompile = "0x0000000000000000000000000000000000000015";
const addressFrom = "0x6Be02d1d3665660d22FF9624b7BE0551ee1Ac91b";
// substrate: '5ELRpquT7C3mWtjeqFMYqgNbcNgWKSr3mYtVi1Uvtc2R7YEx';
const addressTo = "0xAa01a1bEF0557fa9625581a293F3AA7770192632";
// substrate: '2qSbd2umtD4KmV2X7kfttbP8HH4tzL5iMKETbjY2vYXMHHQs';
const addressTo2 = "0x44b21a4e1c4a510237c577c936fba2d6153d2fe2";
// substrate: '5ELRpquT7C3mWtjepSRH3V2At1xp7MA7mb4uuVNsLDiAWZju';
const privKey = "99B3C12287537E38C90A9219D4CB074A89A16E9CDB20BF85728EBD97C343E342";

describe("Test Transfer Balance", function () {
	it("Check balance before transfer", async function () {
		const balanceFrom = web3.utils.fromWei(await web3.eth.getBalance(addressFrom), "ether");
		const balanceTo = await web3.utils.fromWei(await web3.eth.getBalance(addressTo), "ether");

		expect(balanceFrom).to.be.equal("123.45678900000000009");
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
				gas: "5000000000",
				gas_price: 1,
			},
			privKey
		);

		const createReceipt = await web3.eth.sendSignedTransaction(
			createTransaction.rawTransaction
		);

		expect(createReceipt.transactionHash).to.be.equal(
			"0x820524e7e2797a1b231f06b66049de173e1e09a6b61c1ee1373434b41fb29554"
		);
	}).timeout(10000);

	it("Check balance after transfer 10 ether", async function () {
		const balanceFrom = web3.utils.fromWei(await web3.eth.getBalance(addressFrom), "ether");
		const balanceTo = await web3.utils.fromWei(await web3.eth.getBalance(addressTo), "ether");

		expect(balanceFrom).to.be.equal("113.45678899999997909");
		expect(balanceTo).to.be.equal("10");
		expect(await web3.eth.getTransactionCount(addressFrom, "latest")).to.eq(1);
	});

	it("Transfer 100 wei", async function () {
		const createTransaction = await web3.eth.accounts.signTransaction(
			{
				from: addressFrom,
				to: addressTo2,
				value: web3.utils.toWei("100", "wei"),
				gas: "5000000000",
				gas_price: 1,
			},
			privKey
		);
		const createReceipt = await web3.eth.sendSignedTransaction(
			createTransaction.rawTransaction
		);
	}).timeout(10000);

	it("Check balance after transfer 100 wei", async function () {
		const balanceFrom = web3.utils.fromWei(await web3.eth.getBalance(addressFrom), "ether");
		const balanceTo = await web3.utils.fromWei(await web3.eth.getBalance(addressTo2), "ether");

		expect(balanceFrom).to.be.equal("113.45678899999995799");
		expect(balanceTo).to.be.equal("0.0000000000000001");
		expect(await web3.eth.getTransactionCount(addressFrom, "latest")).to.eq(2);
	});

	it("Transfer 50 ether", async function () {
		const createTransaction = await web3.eth.accounts.signTransaction(
			{
				from: addressFrom,
				to: addressTo,
				value: web3.utils.toWei("50", "ether"),
				gas: "5000000000",
				gas_price: 1,
			},
			privKey
		);

		const createReceipt = await web3.eth.sendSignedTransaction(
			createTransaction.rawTransaction
		);

		expect(createReceipt.transactionHash).to.be.equal(
			"0x90febfff3a70204babfa857cb9baa9664badf56f826e696335ef12135d06a89f"
		);
	}).timeout(10000);

	it("Check balance after transfer 50 ether", async function () {
		const balanceFrom = web3.utils.fromWei(await web3.eth.getBalance(addressFrom), "ether");
		const balanceTo = await web3.utils.fromWei(await web3.eth.getBalance(addressTo), "ether");

		expect(balanceFrom).to.be.equal("63.45678899999993699");
		expect(balanceTo).to.be.equal("60");
		expect(await web3.eth.getTransactionCount(addressFrom, "latest")).to.eq(3);
	});

	it("Transfer self", async function () {
		const createTransaction = await web3.eth.accounts.signTransaction(
			{
				from: addressFrom,
				to: addressFrom,
				value: web3.utils.toWei("30", "ether"),
				gas: "5000000000",
				gas_price: 1,
			},
			privKey
		);

		const createReceipt = await web3.eth.sendSignedTransaction(
			createTransaction.rawTransaction
		);
		expect(createReceipt.transactionHash).to.be.equal(
			"0x93b9bc903778f59d124da6ee112b0a5728ab911d00ea198788670be699dec4bf"
		);
	}).timeout(10000);

	it("Check balance after transfer self", async function () {
		const balanceFrom = web3.utils.fromWei(await web3.eth.getBalance(addressFrom), "ether");
		expect(balanceFrom).to.be.equal("63.45678899999991599");
		expect(await web3.eth.getTransactionCount(addressFrom, "latest")).to.eq(4);
	});

	it("Withdraw value from sender", async function () {
		// target address = "723908ee9dc8e509d4b93251bd57f68c09bd9d04471c193fabd8f26c54284a4b(5EeUFyFjHsCJB8TaGXi1PkMgqkxMctcxw8hvfmNdCYGC76xj)";
		const input = "723908ee9dc8e509d4b93251bd57f68c09bd9d04471c193fabd8f26c54284a4b";
		const createTransaction = await web3.eth.accounts.signTransaction(
			{
				from: addressFrom,
				to: addressWithdrawPrecompile,
				gas: "5000000000",
				data: input,
				value: web3.utils.toWei("30", "ether"),
				gas_price: 1,
			},
			privKey
		);
		const createReceipt = await web3.eth.sendSignedTransaction(
			createTransaction.rawTransaction
		);
	}).timeout(10000);

	it("Get sender balance after withdraw", async function () {
		const balanceFrom = web3.utils.fromWei(await web3.eth.getBalance(addressFrom), "ether");
		expect(balanceFrom).to.be.equal("33.456788999999884478");
		expect(await web3.eth.getTransactionCount(addressFrom, "latest")).to.eq(5);
	});
});
