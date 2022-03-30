import path from "path";
import fs from "fs";
import solc from "solc";

// Compile contract
const contractPath = path.resolve(__dirname, "Incrementer.sol");
const source = fs.readFileSync(contractPath, "utf8");
const input = {
	language: "Solidity",
	sources: {
		"Incrementer.sol": {
			content: source,
		},
	},
	settings: {
		outputSelection: {
			"*": {
				"*": ["*"],
			},
		},
	},
};

const tempFile = JSON.parse(solc.compile(JSON.stringify(input)));
export const contractFile = tempFile.contracts["Incrementer.sol"]["Incrementer"];
