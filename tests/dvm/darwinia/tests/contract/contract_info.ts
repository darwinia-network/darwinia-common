// Opcodes.sol
export const opcodes_test = {
	bytecode:
		"0x608060405234801561001057600080fd5b5060405161001d9061007e565b604051809103906000f080158015610039573d6000803e3d6000fd5b506000806101000a81548173ffffffffffffffffffffffffffffffffffffffff021916908373ffffffffffffffffffffffffffffffffffffffff16021790555061008b565b6101438061052283390190565b6104888061009a6000396000f3fe608060405234801561001057600080fd5b506004361061004c5760003560e01c806355313dea146100515780636d3d14161461005b578063b9d1e5aa14610065578063f8a8fd6d1461006f575b600080fd5b610059610079565b005b61006361007b565b005b61006d610080565b005b610077610082565b005b005b600080fd5bfe5b600160021a6002f35b600581101561009f5760018101905061008b565b5060065b60058111156100b7576001810190506100a3565b5060015b60058112156100cf576001810190506100bb565b5060065b60058113156100e7576001810190506100d3565b506002156100f457600051505b60405160208101602060048337505060405160208101602060048339505060405160208101602060048360003c50503660005b8181101561013e5760028152600181019050610127565b505060008020506000602060403e6010608060106040610123612710fa506020610123600af05060008060009054906101000a900473ffffffffffffffffffffffffffffffffffffffff169050600060405180807f697353616d654164647265737328616464726573732c61646472657373290000815250601e01905060405180910390209050600033905060405182815281600482015281602482015260648101604052602081604483600088611388f1505060405182815281600482015281602482015260648101604052602081604483600088611388f250506040518281528160048201528160248201526064810160405260208160448387611388f4505060006242004290507f50cb9fe53daa9737b786ab3646f04d0150dc50ef4e75f59509d83667ad5adb2060001b6040518082815260200191505060405180910390a07f50cb9fe53daa9737b786ab3646f04d0150dc50ef4e75f59509d83667ad5adb2060001b7f50cb9fe53daa9737b786ab3646f04d0150dc50ef4e75f59509d83667ad5adb2060001b6040518082815260200191505060405180910390a13373ffffffffffffffffffffffffffffffffffffffff1660001b7f50cb9fe53daa9737b786ab3646f04d0150dc50ef4e75f59509d83667ad5adb2060001b7f50cb9fe53daa9737b786ab3646f04d0150dc50ef4e75f59509d83667ad5adb2060001b6040518082815260200191505060405180910390a28060001b3373ffffffffffffffffffffffffffffffffffffffff1660001b7f50cb9fe53daa9737b786ab3646f04d0150dc50ef4e75f59509d83667ad5adb2060001b7f50cb9fe53daa9737b786ab3646f04d0150dc50ef4e75f59509d83667ad5adb2060001b6040518082815260200191505060405180910390a38060001b8160001b3373ffffffffffffffffffffffffffffffffffffffff1660001b7f50cb9fe53daa9737b786ab3646f04d0150dc50ef4e75f59509d83667ad5adb2060001b7f50cb9fe53daa9737b786ab3646f04d0150dc50ef4e75f59509d83667ad5adb2060001b6040518082815260200191505060405180910390a46002fffea265627a7a72315820333f079eb993b7f58984bbf9545c72a2e20aff663fbffc8f1fbc0377dc8ef8d064736f6c63430005110032608060405234801561001057600080fd5b50610123806100206000396000f3fe6080604052348015600f57600080fd5b506004361060285760003560e01c8063161e715014602d575b600080fd5b608c60048036036040811015604157600080fd5b81019080803573ffffffffffffffffffffffffffffffffffffffff169060200190929190803573ffffffffffffffffffffffffffffffffffffffff16906020019092919050505060a6565b604051808215151515815260200191505060405180910390f35b60008173ffffffffffffffffffffffffffffffffffffffff168373ffffffffffffffffffffffffffffffffffffffff16141560e3576001905060e8565b600090505b9291505056fea265627a7a723158209aa763fb33ffd981bbff5245958e1f768b12115f2d76ed691b2207d9a5b8711764736f6c63430005110032",
	opcodes:
		"PUSH1 0x80 PUSH1 0x40 MSTORE CALLVALUE DUP1 ISZERO PUSH2 0x10 JUMPI PUSH1 0x0 DUP1 REVERT JUMPDEST POP PUSH1 0x40 MLOAD PUSH2 0x1D SWAP1 PUSH2 0x5F JUMP JUMPDEST PUSH1 0x40 MLOAD DUP1 SWAP2 SUB SWAP1 PUSH1 0x0 CREATE DUP1 ISZERO DUP1 ISZERO PUSH2 0x39 JUMPI RETURNDATASIZE PUSH1 0x0 DUP1 RETURNDATACOPY RETURNDATASIZE PUSH1 0x0 REVERT JUMPDEST POP PUSH1 0x0 DUP1 SLOAD PUSH1 0x1 PUSH1 0x1 PUSH1 0xA0 SHL SUB NOT AND PUSH1 0x1 PUSH1 0x1 PUSH1 0xA0 SHL SUB SWAP3 SWAP1 SWAP3 AND SWAP2 SWAP1 SWAP2 OR SWAP1 SSTORE PUSH2 0x6B JUMP JUMPDEST PUSH1 0xEC DUP1 PUSH2 0x36F DUP4 CODECOPY ADD SWAP1 JUMP JUMPDEST PUSH2 0x2F5 DUP1 PUSH2 0x7A PUSH1 0x0 CODECOPY PUSH1 0x0 RETURN INVALID PUSH1 0x80 PUSH1 0x40 MSTORE CALLVALUE DUP1 ISZERO PUSH2 0x10 JUMPI PUSH1 0x0 DUP1 REVERT JUMPDEST POP PUSH1 0x4 CALLDATASIZE LT PUSH2 0x4C JUMPI PUSH1 0x0 CALLDATALOAD PUSH1 0xE0 SHR DUP1 PUSH4 0x55313DEA EQ PUSH2 0x51 JUMPI DUP1 PUSH4 0x6D3D1416 EQ PUSH2 0x57 JUMPI DUP1 PUSH4 0xB9D1E5AA EQ PUSH2 0x5F JUMPI DUP1 PUSH4 0xF8A8FD6D EQ PUSH2 0x67 JUMPI JUMPDEST PUSH1 0x0 DUP1 REVERT JUMPDEST PUSH2 0x55 JUMPDEST STOP JUMPDEST PUSH2 0x55 PUSH2 0x4C JUMP JUMPDEST PUSH2 0x55 PUSH2 0x6F JUMP JUMPDEST PUSH2 0x55 PUSH2 0x55 JUMP JUMPDEST INVALID JUMPDEST PUSH1 0x5 DUP2 LT ISZERO PUSH2 0x82 JUMPI PUSH1 0x1 ADD PUSH2 0x71 JUMP JUMPDEST POP PUSH1 0x6 JUMPDEST PUSH1 0x5 DUP2 GT ISZERO PUSH2 0x97 JUMPI PUSH1 0x1 ADD PUSH2 0x86 JUMP JUMPDEST POP PUSH1 0x1 JUMPDEST PUSH1 0x5 DUP2 SLT ISZERO PUSH2 0xAC JUMPI PUSH1 0x1 ADD PUSH2 0x9B JUMP JUMPDEST POP PUSH1 0x6 JUMPDEST PUSH1 0x5 DUP2 SGT ISZERO PUSH2 0xC1 JUMPI PUSH1 0x1 ADD PUSH2 0xB0 JUMP JUMPDEST POP PUSH1 0x40 MLOAD PUSH1 0x20 DUP2 ADD PUSH1 0x20 PUSH1 0x4 DUP4 CALLDATACOPY POP POP PUSH1 0x40 MLOAD PUSH1 0x20 DUP2 ADD PUSH1 0x20 PUSH1 0x4 DUP4 CODECOPY POP POP PUSH1 0x40 MLOAD PUSH1 0x20 DUP2 ADD PUSH1 0x20 PUSH1 0x4 DUP4 PUSH1 0x0 EXTCODECOPY POP POP CALLDATASIZE PUSH1 0x0 JUMPDEST DUP2 DUP2 LT ISZERO PUSH2 0x109 JUMPI PUSH1 0x2 DUP2 MSTORE PUSH1 0x1 ADD PUSH2 0xF5 JUMP JUMPDEST POP PUSH1 0x0 SWAP1 POP PUSH1 0x20 PUSH1 0x40 RETURNDATACOPY PUSH1 0x10 PUSH1 0x80 PUSH1 0x10 PUSH1 0x40 PUSH2 0x123 PUSH2 0x2710 STATICCALL POP PUSH1 0x20 PUSH2 0x123 PUSH1 0xA CREATE POP PUSH1 0x0 DUP1 SLOAD PUSH1 0x40 DUP1 MLOAD PUSH32 0x697353616D654164647265737328616464726573732C61646472657373290000 DUP2 MSTORE DUP2 MLOAD SWAP1 DUP2 SWAP1 SUB PUSH1 0x1E ADD DUP2 KECCAK256 DUP1 DUP3 MSTORE CALLER PUSH1 0x4 DUP4 ADD DUP2 SWAP1 MSTORE PUSH1 0x24 DUP4 ADD DUP2 SWAP1 MSTORE PUSH1 0x64 DUP4 ADD SWAP1 SWAP4 MSTORE PUSH1 0x1 PUSH1 0x1 PUSH1 0xA0 SHL SUB SWAP1 SWAP4 AND SWAP4 PUSH1 0x20 SWAP1 DUP3 SWAP1 PUSH1 0x44 SWAP1 DUP3 SWAP1 DUP9 PUSH2 0x1388 CALL POP POP PUSH1 0x40 MLOAD DUP3 DUP2 MSTORE DUP2 PUSH1 0x4 DUP3 ADD MSTORE DUP2 PUSH1 0x24 DUP3 ADD MSTORE PUSH1 0x64 DUP2 ADD PUSH1 0x40 MSTORE PUSH1 0x20 DUP2 PUSH1 0x44 DUP4 PUSH1 0x0 DUP9 PUSH2 0x1388 CALLCODE POP POP PUSH1 0x40 MLOAD DUP3 DUP2 MSTORE DUP2 PUSH1 0x4 DUP3 ADD MSTORE DUP2 PUSH1 0x24 DUP3 ADD MSTORE PUSH1 0x64 DUP2 ADD PUSH1 0x40 MSTORE PUSH1 0x20 DUP2 PUSH1 0x44 DUP4 DUP8 PUSH2 0x1388 DELEGATECALL POP POP PUSH1 0x40 DUP1 MLOAD PUSH1 0x0 DUP1 MLOAD PUSH1 0x20 PUSH2 0x2A1 DUP4 CODECOPY DUP2 MLOAD SWAP2 MSTORE DUP2 MSTORE SWAP1 MLOAD PUSH3 0x420042 SWAP2 DUP2 SWAP1 SUB PUSH1 0x20 ADD SWAP1 LOG0 PUSH1 0x40 DUP1 MLOAD PUSH1 0x0 DUP1 MLOAD PUSH1 0x20 PUSH2 0x2A1 DUP4 CODECOPY DUP2 MLOAD SWAP2 MSTORE DUP1 DUP3 MSTORE SWAP2 MLOAD SWAP1 DUP2 SWAP1 SUB PUSH1 0x20 ADD SWAP1 LOG1 PUSH1 0x40 DUP1 MLOAD PUSH1 0x0 DUP1 MLOAD PUSH1 0x20 PUSH2 0x2A1 DUP4 CODECOPY DUP2 MLOAD SWAP2 MSTORE DUP1 DUP3 MSTORE SWAP2 MLOAD CALLER SWAP3 SWAP2 DUP2 SWAP1 SUB PUSH1 0x20 ADD SWAP1 LOG2 PUSH1 0x40 DUP1 MLOAD PUSH1 0x0 DUP1 MLOAD PUSH1 0x20 PUSH2 0x2A1 DUP4 CODECOPY DUP2 MLOAD SWAP2 MSTORE DUP1 DUP3 MSTORE SWAP2 MLOAD DUP4 SWAP3 CALLER SWAP3 SWAP1 SWAP2 SWAP1 DUP2 SWAP1 SUB PUSH1 0x20 ADD SWAP1 LOG3 PUSH1 0x40 DUP1 MLOAD PUSH1 0x0 DUP1 MLOAD PUSH1 0x20 PUSH2 0x2A1 DUP4 CODECOPY DUP2 MLOAD SWAP2 MSTORE DUP1 DUP3 MSTORE SWAP2 MLOAD DUP4 SWAP3 DUP4 SWAP3 CALLER SWAP3 SWAP1 DUP2 SWAP1 SUB PUSH1 0x20 ADD SWAP1 LOG4 PUSH1 0x2 SELFDESTRUCT INVALID POP 0xCB SWAP16 0xE5 RETURNDATASIZE 0xAA SWAP8 CALLDATACOPY 0xB7 DUP7 0xAB CALLDATASIZE CHAINID CREATE 0x4D ADD POP 0xDC POP 0xEF 0x4E PUSH22 0xF59509D83667AD5ADB20A265627A7A7231582025F876 0xDC RETURNDATACOPY 0xDF PUSH21 0x5928CF90A2E6207754F1A047218863FA8959868B4A NOT SWAP16 LOG3 PUSH13 0x64736F6C634300051100326080 PUSH1 0x40 MSTORE CALLVALUE DUP1 ISZERO PUSH2 0x10 JUMPI PUSH1 0x0 DUP1 REVERT JUMPDEST POP PUSH1 0xCD DUP1 PUSH2 0x1F PUSH1 0x0 CODECOPY PUSH1 0x0 RETURN INVALID PUSH1 0x80 PUSH1 0x40 MSTORE CALLVALUE DUP1 ISZERO PUSH1 0xF JUMPI PUSH1 0x0 DUP1 REVERT JUMPDEST POP PUSH1 0x4 CALLDATASIZE LT PUSH1 0x28 JUMPI PUSH1 0x0 CALLDATALOAD PUSH1 0xE0 SHR DUP1 PUSH4 0x161E7150 EQ PUSH1 0x2D JUMPI JUMPDEST PUSH1 0x0 DUP1 REVERT JUMPDEST PUSH1 0x58 PUSH1 0x4 DUP1 CALLDATASIZE SUB PUSH1 0x40 DUP2 LT ISZERO PUSH1 0x41 JUMPI PUSH1 0x0 DUP1 REVERT JUMPDEST POP PUSH1 0x1 PUSH1 0x1 PUSH1 0xA0 SHL SUB DUP2 CALLDATALOAD DUP2 AND SWAP2 PUSH1 0x20 ADD CALLDATALOAD AND PUSH1 0x6C JUMP JUMPDEST PUSH1 0x40 DUP1 MLOAD SWAP2 ISZERO ISZERO DUP3 MSTORE MLOAD SWAP1 DUP2 SWAP1 SUB PUSH1 0x20 ADD SWAP1 RETURN JUMPDEST PUSH1 0x0 DUP2 PUSH1 0x1 PUSH1 0x1 PUSH1 0xA0 SHL SUB AND DUP4 PUSH1 0x1 PUSH1 0x1 PUSH1 0xA0 SHL SUB AND EQ ISZERO PUSH1 0x8E JUMPI POP PUSH1 0x1 PUSH1 0x92 JUMP JUMPDEST POP PUSH1 0x0 JUMPDEST SWAP3 SWAP2 POP POP JUMP INVALID LOG2 PUSH6 0x627A7A723158 KECCAK256 0xE6 SHL 0xED 0xDB 0xED 0xA6 PUSH11 0x98E4316F08138D6DDF7BF6 0xDA COINBASE SAR DUP8 0xD 0xD8 DUP5 0xBF SSTORE 0xC SDIV 0x5D 0xEF PUSH30 0x64736F6C6343000511003200000000000000000000000000000000000000 ",
	abi: [
		{
			inputs: [],
			payable: false,
			stateMutability: "nonpayable",
			type: "constructor",
		},
		{
			constant: false,
			inputs: [],
			name: "test",
			outputs: [],
			payable: false,
			stateMutability: "nonpayable",
			type: "function",
		},
		{
			constant: false,
			inputs: [],
			name: "test_invalid",
			outputs: [],
			payable: false,
			stateMutability: "nonpayable",
			type: "function",
		},
		{
			constant: false,
			inputs: [],
			name: "test_revert",
			outputs: [],
			payable: false,
			stateMutability: "nonpayable",
			type: "function",
		},
		{
			constant: false,
			inputs: [],
			name: "test_stop",
			outputs: [],
			payable: false,
			stateMutability: "nonpayable",
			type: "function",
		},
	],
};

// TestLog.sol
export const log_test = {
	bytecode:
		"0x608060405234801561001057600080fd5b506108db806100206000396000f3fe608060405234801561001057600080fd5b50600436106101585760003560e01c80639a19a953116100c3578063d2282dc51161007c578063d2282dc514610381578063e30081a0146103af578063e8beef5b146103f3578063f38b0600146103fd578063f5b53e1714610407578063fd4087671461042557610158565b80639a19a953146102d65780639dc2c8f514610307578063a53b1c1e14610311578063a67808571461033f578063b61c050314610349578063c2b12a731461035357610158565b806338cc48311161011557806338cc48311461022c5780634e7ad3671461027657806357cb2fc41461028057806365538c73146102a457806368895979146102ae57806376bc21d9146102cc57610158565b8063102accc11461015d57806312a7b914146101675780631774e646146101895780631e26fd33146101ba5780631f903037146101ea578063343a875d14610208575b600080fd5b61016561042f565b005b61016f610484565b604051808215151515815260200191505060405180910390f35b6101b86004803603602081101561019f57600080fd5b81019080803560ff16906020019092919050505061049a565b005b6101e8600480360360208110156101d057600080fd5b810190808035151590602001909291905050506104b8565b005b6101f26104d4565b6040518082815260200191505060405180910390f35b6102106104de565b604051808260ff1660ff16815260200191505060405180910390f35b6102346104f4565b604051808273ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200191505060405180910390f35b61027e61051e565b005b61028861053b565b604051808260000b60000b815260200191505060405180910390f35b6102ac610551565b005b6102b661058b565b6040518082815260200191505060405180910390f35b6102d4610595565b005b610305600480360360208110156102ec57600080fd5b81019080803560000b90602001909291905050506105c9565b005b61030f6105ea565b005b61033d6004803603602081101561032757600080fd5b810190808035906020019092919050505061066d565b005b610347610677565b005b610351610690565b005b61037f6004803603602081101561036957600080fd5b81019080803590602001909291905050506106ce565b005b6103ad6004803603602081101561039757600080fd5b81019080803590602001909291905050506106d8565b005b6103f1600480360360208110156103c557600080fd5b81019080803573ffffffffffffffffffffffffffffffffffffffff1690602001909291905050506106e2565b005b6103fb610726565b005b61040561077e565b005b61040f6107f7565b6040518082815260200191505060405180910390f35b61042d610801565b005b3373ffffffffffffffffffffffffffffffffffffffff16600115157f0e216b62efbb97e751a2ce09f607048751720397ecfb9eef1e48a6644948985b602a6040518082815260200191505060405180910390a3565b60008060009054906101000a900460ff16905090565b80600060026101000a81548160ff021916908360ff16021790555050565b806000806101000a81548160ff02191690831515021790555050565b6000600454905090565b60008060029054906101000a900460ff16905090565b6000600360009054906101000a900473ffffffffffffffffffffffffffffffffffffffff16905090565b60011515602a6040518082815260200191505060405180910390a1565b60008060019054906101000a900460000b905090565b7f65c9ac8011e286e89d02a269890f41d67ca2cc597b2c76c7c69321ff492be580602a6040518082815260200191505060405180910390a1565b6000600254905090565b3373ffffffffffffffffffffffffffffffffffffffff1660011515602a6040518082815260200191505060405180910390a2565b80600060016101000a81548160ff021916908360000b60ff16021790555050565b7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff60001b3373ffffffffffffffffffffffffffffffffffffffff16600115157fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffe9602a604051808360000b81526020018281526020019250505060405180910390a3565b8060018190555050565b602a6040518082815260200191505060405180910390a0565b600115157f81933b308056e7e85668661dcd102b1f22795b4431f9cf4625794f381c271c6b602a6040518082815260200191505060405180910390a2565b8060048190555050565b8060028190555050565b80600360006101000a81548173ffffffffffffffffffffffffffffffffffffffff021916908373ffffffffffffffffffffffffffffffffffffffff16021790555050565b7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff60001b3373ffffffffffffffffffffffffffffffffffffffff1660011515602a6040518082815260200191505060405180910390a3565b7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff60001b3373ffffffffffffffffffffffffffffffffffffffff16600115157f317b31292193c2a4f561cc40a95ea0d97a2733f14af6d6d59522473e1f3ae65f602a6040518082815260200191505060405180910390a4565b6000600154905090565b7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff60001b3373ffffffffffffffffffffffffffffffffffffffff16600115157fd5f0a30e4be0c6be577a71eceb7464245a796a7e6a55c0d971837b250de05f4e7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffe9602a604051808360000b81526020018281526020019250505060405180910390a456fea2646970667358221220577f07990960d4d95c9523bf1d85b8b6d97ccbb8ff276b695dce59e5fcaf619b64736f6c63430006000033",
	abi: [
		{
			inputs: [],
			stateMutability: "nonpayable",
			type: "constructor",
		},
		{
			anonymous: false,
			inputs: [
				{
					indexed: false,
					internalType: "uint256",
					name: "value",
					type: "uint256",
				},
			],
			name: "Log0",
			type: "event",
		},
		{
			anonymous: true,
			inputs: [
				{
					indexed: false,
					internalType: "uint256",
					name: "value",
					type: "uint256",
				},
			],
			name: "Log0Anonym",
			type: "event",
		},
		{
			anonymous: false,
			inputs: [
				{
					indexed: true,
					internalType: "bool",
					name: "aBool",
					type: "bool",
				},
				{
					indexed: false,
					internalType: "uint256",
					name: "value",
					type: "uint256",
				},
			],
			name: "Log1",
			type: "event",
		},
		{
			anonymous: true,
			inputs: [
				{
					indexed: true,
					internalType: "bool",
					name: "aBool",
					type: "bool",
				},
				{
					indexed: false,
					internalType: "uint256",
					name: "value",
					type: "uint256",
				},
			],
			name: "Log1Anonym",
			type: "event",
		},
		{
			anonymous: false,
			inputs: [
				{
					indexed: true,
					internalType: "bool",
					name: "aBool",
					type: "bool",
				},
				{
					indexed: true,
					internalType: "address",
					name: "aAddress",
					type: "address",
				},
				{
					indexed: false,
					internalType: "uint256",
					name: "value",
					type: "uint256",
				},
			],
			name: "Log2",
			type: "event",
		},
		{
			anonymous: true,
			inputs: [
				{
					indexed: true,
					internalType: "bool",
					name: "aBool",
					type: "bool",
				},
				{
					indexed: true,
					internalType: "address",
					name: "aAddress",
					type: "address",
				},
				{
					indexed: false,
					internalType: "uint256",
					name: "value",
					type: "uint256",
				},
			],
			name: "Log2Anonym",
			type: "event",
		},
		{
			anonymous: false,
			inputs: [
				{
					indexed: true,
					internalType: "bool",
					name: "aBool",
					type: "bool",
				},
				{
					indexed: true,
					internalType: "address",
					name: "aAddress",
					type: "address",
				},
				{
					indexed: true,
					internalType: "bytes32",
					name: "aBytes32",
					type: "bytes32",
				},
				{
					indexed: false,
					internalType: "uint256",
					name: "value",
					type: "uint256",
				},
			],
			name: "Log3",
			type: "event",
		},
		{
			anonymous: true,
			inputs: [
				{
					indexed: true,
					internalType: "bool",
					name: "aBool",
					type: "bool",
				},
				{
					indexed: true,
					internalType: "address",
					name: "aAddress",
					type: "address",
				},
				{
					indexed: true,
					internalType: "bytes32",
					name: "aBytes32",
					type: "bytes32",
				},
				{
					indexed: false,
					internalType: "uint256",
					name: "value",
					type: "uint256",
				},
			],
			name: "Log3Anonym",
			type: "event",
		},
		{
			anonymous: false,
			inputs: [
				{
					indexed: true,
					internalType: "bool",
					name: "aBool",
					type: "bool",
				},
				{
					indexed: true,
					internalType: "address",
					name: "aAddress",
					type: "address",
				},
				{
					indexed: true,
					internalType: "bytes32",
					name: "aBytes32",
					type: "bytes32",
				},
				{
					indexed: false,
					internalType: "int8",
					name: "aInt8",
					type: "int8",
				},
				{
					indexed: false,
					internalType: "uint256",
					name: "value",
					type: "uint256",
				},
			],
			name: "Log4",
			type: "event",
		},
		{
			anonymous: true,
			inputs: [
				{
					indexed: true,
					internalType: "bool",
					name: "aBool",
					type: "bool",
				},
				{
					indexed: true,
					internalType: "address",
					name: "aAddress",
					type: "address",
				},
				{
					indexed: true,
					internalType: "bytes32",
					name: "aBytes32",
					type: "bytes32",
				},
				{
					indexed: false,
					internalType: "int8",
					name: "aInt8",
					type: "int8",
				},
				{
					indexed: false,
					internalType: "uint256",
					name: "value",
					type: "uint256",
				},
			],
			name: "Log4Anonym",
			type: "event",
		},
		{
			inputs: [],
			name: "fireEventLog0",
			outputs: [],
			stateMutability: "nonpayable",
			type: "function",
		},
		{
			inputs: [],
			name: "fireEventLog0Anonym",
			outputs: [],
			stateMutability: "nonpayable",
			type: "function",
		},
		{
			inputs: [],
			name: "fireEventLog1",
			outputs: [],
			stateMutability: "nonpayable",
			type: "function",
		},
		{
			inputs: [],
			name: "fireEventLog1Anonym",
			outputs: [],
			stateMutability: "nonpayable",
			type: "function",
		},
		{
			inputs: [],
			name: "fireEventLog2",
			outputs: [],
			stateMutability: "nonpayable",
			type: "function",
		},
		{
			inputs: [],
			name: "fireEventLog2Anonym",
			outputs: [],
			stateMutability: "nonpayable",
			type: "function",
		},
		{
			inputs: [],
			name: "fireEventLog3",
			outputs: [],
			stateMutability: "nonpayable",
			type: "function",
		},
		{
			inputs: [],
			name: "fireEventLog3Anonym",
			outputs: [],
			stateMutability: "nonpayable",
			type: "function",
		},
		{
			inputs: [],
			name: "fireEventLog4",
			outputs: [],
			stateMutability: "nonpayable",
			type: "function",
		},
		{
			inputs: [],
			name: "fireEventLog4Anonym",
			outputs: [],
			stateMutability: "nonpayable",
			type: "function",
		},
		{
			inputs: [],
			name: "getAddress",
			outputs: [
				{
					internalType: "address",
					name: "ret",
					type: "address",
				},
			],
			stateMutability: "view",
			type: "function",
		},
		{
			inputs: [],
			name: "getBool",
			outputs: [
				{
					internalType: "bool",
					name: "ret",
					type: "bool",
				},
			],
			stateMutability: "view",
			type: "function",
		},
		{
			inputs: [],
			name: "getBytes32",
			outputs: [
				{
					internalType: "bytes32",
					name: "ret",
					type: "bytes32",
				},
			],
			stateMutability: "view",
			type: "function",
		},
		{
			inputs: [],
			name: "getInt256",
			outputs: [
				{
					internalType: "int256",
					name: "ret",
					type: "int256",
				},
			],
			stateMutability: "view",
			type: "function",
		},
		{
			inputs: [],
			name: "getInt8",
			outputs: [
				{
					internalType: "int8",
					name: "ret",
					type: "int8",
				},
			],
			stateMutability: "view",
			type: "function",
		},
		{
			inputs: [],
			name: "getUint256",
			outputs: [
				{
					internalType: "uint256",
					name: "ret",
					type: "uint256",
				},
			],
			stateMutability: "view",
			type: "function",
		},
		{
			inputs: [],
			name: "getUint8",
			outputs: [
				{
					internalType: "uint8",
					name: "ret",
					type: "uint8",
				},
			],
			stateMutability: "view",
			type: "function",
		},
		{
			inputs: [
				{
					internalType: "address",
					name: "_address",
					type: "address",
				},
			],
			name: "setAddress",
			outputs: [],
			stateMutability: "nonpayable",
			type: "function",
		},
		{
			inputs: [
				{
					internalType: "bool",
					name: "_bool",
					type: "bool",
				},
			],
			name: "setBool",
			outputs: [],
			stateMutability: "nonpayable",
			type: "function",
		},
		{
			inputs: [
				{
					internalType: "bytes32",
					name: "_bytes32",
					type: "bytes32",
				},
			],
			name: "setBytes32",
			outputs: [],
			stateMutability: "nonpayable",
			type: "function",
		},
		{
			inputs: [
				{
					internalType: "int256",
					name: "_int256",
					type: "int256",
				},
			],
			name: "setInt256",
			outputs: [],
			stateMutability: "nonpayable",
			type: "function",
		},
		{
			inputs: [
				{
					internalType: "int8",
					name: "_int8",
					type: "int8",
				},
			],
			name: "setInt8",
			outputs: [],
			stateMutability: "nonpayable",
			type: "function",
		},
		{
			inputs: [
				{
					internalType: "uint256",
					name: "_uint256",
					type: "uint256",
				},
			],
			name: "setUint256",
			outputs: [],
			stateMutability: "nonpayable",
			type: "function",
		},
		{
			inputs: [
				{
					internalType: "uint8",
					name: "_uint8",
					type: "uint8",
				},
			],
			name: "setUint8",
			outputs: [],
			stateMutability: "nonpayable",
			type: "function",
		},
	],
};
