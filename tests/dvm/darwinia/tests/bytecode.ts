export const bytecode = {
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
