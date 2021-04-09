const expect = require("chai").expect;
const assert = require("chai").assert;
const Web3 = require("web3");
const conf = require("./config.js");
const web3 = new Web3(conf.host);
web3.eth.accounts.wallet.add(conf.privKey);
// Wkton bytecode
const bytecode =
	"60806040526040805190810160405280600c81526020017f57726170706564204b544f4e00000000000000000000000000000000000000008152506000908051906020019061004f92919061010c565b506040805190810160405280600581526020017f574b544f4e0000000000000000000000000000000000000000000000000000008152506001908051906020019061009b92919061010c565b506012600260006101000a81548160ff021916908360ff1602179055506016600260016101000a81548173ffffffffffffffffffffffffffffffffffffffff021916908373ffffffffffffffffffffffffffffffffffffffff16021790555034801561010657600080fd5b506101b1565b828054600181600116156101000203166002900490600052602060002090601f016020900481019282601f1061014d57805160ff191683800117855561017b565b8280016001018555821561017b579182015b8281111561017a57825182559160200191906001019061015f565b5b509050610188919061018c565b5090565b6101ae91905b808211156101aa576000816000905550600101610192565b5090565b90565b610f3680620001c16000396000f3006080604052600436106100ba576000357c0100000000000000000000000000000000000000000000000000000000900463ffffffff168063040cf020146100bf57806306fdde03146100fa578063095ea7b31461018a57806318160ddd146101ef57806323b872dd1461021a578063313ce5671461029f57806347e7ef24146102d057806370a082311461031d57806395d89b4114610374578063a9059cbb14610404578063b548602014610469578063dd62ed3e146104c0575b600080fd5b3480156100cb57600080fd5b506100f8600480360381019080803560001916906020019092919080359060200190929190505050610537565b005b34801561010657600080fd5b5061010f610786565b6040518080602001828103825283818151815260200191508051906020019080838360005b8381101561014f578082015181840152602081019050610134565b50505050905090810190601f16801561017c5780820380516001836020036101000a031916815260200191505b509250505060405180910390f35b34801561019657600080fd5b506101d5600480360381019080803573ffffffffffffffffffffffffffffffffffffffff16906020019092919080359060200190929190505050610824565b604051808215151515815260200191505060405180910390f35b3480156101fb57600080fd5b50610204610916565b6040518082815260200191505060405180910390f35b34801561022657600080fd5b50610285600480360381019080803573ffffffffffffffffffffffffffffffffffffffff169060200190929190803573ffffffffffffffffffffffffffffffffffffffff16906020019092919080359060200190929190505050610920565b604051808215151515815260200191505060405180910390f35b3480156102ab57600080fd5b506102b4610c6d565b604051808260ff1660ff16815260200191505060405180910390f35b3480156102dc57600080fd5b5061031b600480360381019080803573ffffffffffffffffffffffffffffffffffffffff16906020019092919080359060200190929190505050610c80565b005b34801561032957600080fd5b5061035e600480360381019080803573ffffffffffffffffffffffffffffffffffffffff169060200190929190505050610df4565b6040518082815260200191505060405180910390f35b34801561038057600080fd5b50610389610e0c565b6040518080602001828103825283818151815260200191508051906020019080838360005b838110156103c95780820151818401526020810190506103ae565b50505050905090810190601f1680156103f65780820380516001836020036101000a031916815260200191505b509250505060405180910390f35b34801561041057600080fd5b5061044f600480360381019080803573ffffffffffffffffffffffffffffffffffffffff16906020019092919080359060200190929190505050610eaa565b604051808215151515815260200191505060405180910390f35b34801561047557600080fd5b5061047e610ebf565b604051808273ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200191505060405180910390f35b3480156104cc57600080fd5b50610521600480360381019080803573ffffffffffffffffffffffffffffffffffffffff169060200190929190803573ffffffffffffffffffffffffffffffffffffffff169060200190929190505050610ee5565b6040518082815260200191505060405180910390f35b600081600460003373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff168152602001908152602001600020541015151561058757600080fd5b8160036000828254039250508190555081600460003373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160002060008282540392505081905550600260019054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff1660405180807f776974686472617728627974657333322c75696e743235362900000000000000815250601901905060405180910390207c0100000000000000000000000000000000000000000000000000000000900484846040518363ffffffff167c0100000000000000000000000000000000000000000000000000000000028152600401808360001916600019168152602001828152602001925050506000604051808303816000875af1925050509050801515610745576040517f08c379a00000000000000000000000000000000000000000000000000000000081526004018080602001828103825260168152602001807f574b544f4e3a2057495448445241575f4641494c45440000000000000000000081525060200191505060405180910390fd5b82600019167fa4dfdde26c326c8cced668e6a665f4efc3f278bdc9101cdedc4f725abd63a1ee836040518082815260200191505060405180910390a2505050565b60008054600181600116156101000203166002900480601f01602080910402602001604051908101604052809291908181526020018280546001816001161561010002031660029004801561081c5780601f106107f15761010080835404028352916020019161081c565b820191906000526020600020905b8154815290600101906020018083116107ff57829003601f168201915b505050505081565b600081600560003373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160002060008573ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff168152602001908152602001600020819055508273ffffffffffffffffffffffffffffffffffffffff163373ffffffffffffffffffffffffffffffffffffffff167f8c5be1e5ebec7d5bd14f71427d1e84f3dd0314c0f7b2291e5b200ac8c7c3b925846040518082815260200191505060405180910390a36001905092915050565b6000600354905090565b600081600460008673ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff168152602001908152602001600020541015151561097057600080fd5b3373ffffffffffffffffffffffffffffffffffffffff168473ffffffffffffffffffffffffffffffffffffffff1614158015610a4857507fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff600560008673ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160002060003373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff1681526020019081526020016000205414155b15610b635781600560008673ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160002060003373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff1681526020019081526020016000205410151515610ad857600080fd5b81600560008673ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160002060003373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff168152602001908152602001600020600082825403925050819055505b81600460008673ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff1681526020019081526020016000206000828254039250508190555081600460008573ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff168152602001908152602001600020600082825401925050819055508273ffffffffffffffffffffffffffffffffffffffff168473ffffffffffffffffffffffffffffffffffffffff167fddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef846040518082815260200191505060405180910390a3600190509392505050565b600260009054906101000a900460ff1681565b600260019054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff163373ffffffffffffffffffffffffffffffffffffffff16141515610d45576040517f08c379a00000000000000000000000000000000000000000000000000000000081526004018080602001828103825260118152602001807f574b544f4e3a205045524d495353494f4e00000000000000000000000000000081525060200191505060405180910390fd5b8060036000828254019250508190555080600460008473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff168152602001908152602001600020600082825401925050819055508173ffffffffffffffffffffffffffffffffffffffff167fe1fffcc4923d04b559f4d29a8bfc6cda04eb5b0d3c460751c2402c5c5cc9109c826040518082815260200191505060405180910390a25050565b60046020528060005260406000206000915090505481565b60018054600181600116156101000203166002900480601f016020809104026020016040519081016040528092919081815260200182805460018160011615610100020316600290048015610ea25780601f10610e7757610100808354040283529160200191610ea2565b820191906000526020600020905b815481529060010190602001808311610e8557829003601f168201915b505050505081565b6000610eb7338484610920565b905092915050565b600260019054906101000a900473ffffffffffffffffffffffffffffffffffffffff1681565b60056020528160005260406000206020528060005260406000206000915091505054815600a165627a7a72305820b92fe8da73c1c47346ad917559ce98eaafbc3323d4a8081f89fa3acc508cff610029";
const abi = [
	{
		constant: false,
		inputs: [
			{
				name: "to",
				type: "bytes32",
			},
			{
				name: "wad",
				type: "uint256",
			},
		],
		name: "withdraw",
		outputs: [],
		payable: false,
		stateMutability: "nonpayable",
		type: "function",
	},
	{
		constant: true,
		inputs: [],
		name: "name",
		outputs: [
			{
				name: "",
				type: "string",
			},
		],
		payable: false,
		stateMutability: "view",
		type: "function",
	},
	{
		constant: false,
		inputs: [
			{
				name: "guy",
				type: "address",
			},
			{
				name: "wad",
				type: "uint256",
			},
		],
		name: "approve",
		outputs: [
			{
				name: "",
				type: "bool",
			},
		],
		payable: false,
		stateMutability: "nonpayable",
		type: "function",
	},
	{
		constant: true,
		inputs: [],
		name: "totalSupply",
		outputs: [
			{
				name: "",
				type: "uint256",
			},
		],
		payable: false,
		stateMutability: "view",
		type: "function",
	},
	{
		constant: false,
		inputs: [
			{
				name: "src",
				type: "address",
			},
			{
				name: "dst",
				type: "address",
			},
			{
				name: "wad",
				type: "uint256",
			},
		],
		name: "transferFrom",
		outputs: [
			{
				name: "",
				type: "bool",
			},
		],
		payable: false,
		stateMutability: "nonpayable",
		type: "function",
	},
	{
		constant: true,
		inputs: [],
		name: "decimals",
		outputs: [
			{
				name: "",
				type: "uint8",
			},
		],
		payable: false,
		stateMutability: "view",
		type: "function",
	},
	{
		constant: false,
		inputs: [
			{
				name: "from",
				type: "address",
			},
			{
				name: "value",
				type: "uint256",
			},
		],
		name: "deposit",
		outputs: [],
		payable: false,
		stateMutability: "nonpayable",
		type: "function",
	},
	{
		constant: true,
		inputs: [
			{
				name: "",
				type: "address",
			},
		],
		name: "balanceOf",
		outputs: [
			{
				name: "",
				type: "uint256",
			},
		],
		payable: false,
		stateMutability: "view",
		type: "function",
	},
	{
		constant: true,
		inputs: [],
		name: "symbol",
		outputs: [
			{
				name: "",
				type: "string",
			},
		],
		payable: false,
		stateMutability: "view",
		type: "function",
	},
	{
		constant: false,
		inputs: [
			{
				name: "dst",
				type: "address",
			},
			{
				name: "wad",
				type: "uint256",
			},
		],
		name: "transfer",
		outputs: [
			{
				name: "",
				type: "bool",
			},
		],
		payable: false,
		stateMutability: "nonpayable",
		type: "function",
	},
	{
		constant: true,
		inputs: [],
		name: "KTON_PRECOMPILE",
		outputs: [
			{
				name: "",
				type: "address",
			},
		],
		payable: false,
		stateMutability: "view",
		type: "function",
	},
	{
		constant: true,
		inputs: [
			{
				name: "",
				type: "address",
			},
			{
				name: "",
				type: "address",
			},
		],
		name: "allowance",
		outputs: [
			{
				name: "",
				type: "uint256",
			},
		],
		payable: false,
		stateMutability: "view",
		type: "function",
	},
	{
		anonymous: false,
		inputs: [
			{
				indexed: true,
				name: "src",
				type: "address",
			},
			{
				indexed: true,
				name: "guy",
				type: "address",
			},
			{
				indexed: false,
				name: "wad",
				type: "uint256",
			},
		],
		name: "Approval",
		type: "event",
	},
	{
		anonymous: false,
		inputs: [
			{
				indexed: true,
				name: "src",
				type: "address",
			},
			{
				indexed: true,
				name: "dst",
				type: "address",
			},
			{
				indexed: false,
				name: "wad",
				type: "uint256",
			},
		],
		name: "Transfer",
		type: "event",
	},
	{
		anonymous: false,
		inputs: [
			{
				indexed: true,
				name: "dst",
				type: "address",
			},
			{
				indexed: false,
				name: "wad",
				type: "uint256",
			},
		],
		name: "Deposit",
		type: "event",
	},
	{
		anonymous: false,
		inputs: [
			{
				indexed: true,
				name: "src",
				type: "bytes32",
			},
			{
				indexed: false,
				name: "wad",
				type: "uint256",
			},
		],
		name: "Withdrawal",
		type: "event",
	},
];
const jsontest = new web3.eth.Contract(abi);
jsontest.options.from = conf.address;
jsontest.options.gas = conf.gas;
const addressTo = "0x0000000000000000000000000000000000000016";

describe("Test Kton Precompile", function () {
	after(() => {
		web3.currentProvider.disconnect();
	});

	it("Deploy kton contract", async function () {
		const instance = await jsontest
			.deploy({
				data: bytecode,
				arguments: [],
			})
			.send();
		jsontest.options.address = instance.options.address;
	}).timeout(10000);

	it("Test Transfer and call 1", async function () {
		// ethabi encode: transfer_and_call(address,uint256)
		// p1: address, C2Bf5F29a4384b1aB0C063e1c666f02121B6084a
		// p2: uint256, 000000000000000000000000000000000000000000000001a055690d9db80000（30_000_000_000_000_000_000）
		var input =
			"3225da29000000000000000000000000" +
			jsontest.options.address.slice(2) +
			"000000000000000000000000000000000000000000000001a055690d9db80000";
		const createTransaction = await web3.eth.accounts.signTransaction(
			{
				from: jsontest.options.from,
				to: addressTo,
				gas: conf.gas,
				data: input,
			},
			conf.privKey
		);

		const createReceipt = await web3.eth.sendSignedTransaction(
			createTransaction.rawTransaction
		);
	}).timeout(80000);

	it("Check after transfer and call 1", async function () {
		var balance = await jsontest.methods.balanceOf(jsontest.options.from).call();
		expect(balance).to.be.equal("30000000000000000000");
		// check apps, the caller kton balance - 30
	}).timeout(80000);

	it("Test Transfer and call 2", async function () {
		// ethabi encode: transfer_and_call(address,uint256)
		// p1: address, C2Bf5F29a4384b1aB0C063e1c666f02121B6084a
		// p2: uint256, 000000000000000000000000000000000000000000000001a055690d9db80000（30_000_000_000_000_000_000）
		var input =
			"3225da29000000000000000000000000" +
			jsontest.options.address.slice(2) +
			"000000000000000000000000000000000000000000000001a055690d9db80000";
		const createTransaction = await web3.eth.accounts.signTransaction(
			{
				from: jsontest.options.from,
				to: addressTo,
				gas: conf.gas,
				data: input,
			},
			conf.privKey
		);

		const createReceipt = await web3.eth.sendSignedTransaction(
			createTransaction.rawTransaction
		);
	}).timeout(80000);

	it("Check after transfer and call 2", async function () {
		var balance = await jsontest.methods.balanceOf(jsontest.options.from).call();
		expect(balance).to.be.equal("60000000000000000000");
		// check apps, the caller kton balance - 30
	}).timeout(80000);

	it("Test Withdraw 10", async function () {
		// substrate 2qSbd2umtD4KmV2X7kfttbP8HH4tzL5iMKETbjY2vYXMHHQs(0xAa01a1bEF0557fa9625581a293F3AA7770192632)
		var withdrawal_address =
			"0x64766d3a00000000000000aa01a1bef0557fa9625581a293f3aa777019263256";
		var result = await jsontest.methods
			.withdraw(withdrawal_address, "10000000000000000000")
			.send({ from: conf.address });

		// var input_deposit_address = "0x6Be02d1d3665660d22FF9624b7BE0551ee1Ac91b";
		var balance = await jsontest.methods.balanceOf(conf.address).call();
		expect(balance).to.be.equal("50000000000000000000");

		// check apps 2qSbd2umtD4KmV2X7kfttbP8HH4tzL5iMKETbjY2vYXMHHQs kton balance = 10
	}).timeout(80000);

	it("Test Withdraw 50", async function () {
		// substrate 2qSbd2umtD4KmV2X7kfttbP8HH4tzL5iMKETbjY2vYXMHHQs(0xAa01a1bEF0557fa9625581a293F3AA7770192632)
		var withdrawal_address =
			"0x64766d3a00000000000000aa01a1bef0557fa9625581a293f3aa777019263256";
		var result = await jsontest.methods
			.withdraw(withdrawal_address, "50000000000000000000")
			.send({ from: conf.address });

		// var input_deposit_address = "0x6Be02d1d3665660d22FF9624b7BE0551ee1Ac91b";
		var balance = await jsontest.methods.balanceOf(conf.address).call();
		expect(balance).to.be.equal("0");

		// check apps 2qSbd2umtD4KmV2X7kfttbP8HH4tzL5iMKETbjY2vYXMHHQs kton balance = 60
	}).timeout(80000);
});
