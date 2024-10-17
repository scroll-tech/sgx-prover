// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.12;

import "forge-std/Script.sol";
import {VmSafe} from "forge-std/Vm.sol";
import {AttestationVerifier} from "src/core/AttestationVerifier.sol";
import {ProverRegistry} from "src/core/ProverRegistry.sol";

contract Deploy is Script {
    function setUp() public {}

    function getOutputFilePath() private view returns (string memory) {
        string memory env = vm.envString("ENV");
        return
            string.concat(
                vm.projectRoot(),
                "/deployment/tee_deploy_",
                env,
                ".json"
            );
    }

    function readJson() private returns (string memory) {
        bytes32 remark = keccak256(abi.encodePacked("remark"));
        string memory output = vm.readFile(getOutputFilePath());
        string[] memory keys = vm.parseJsonKeys(output, ".");
        for (uint i = 0; i < keys.length; i++) {
            if (keccak256(abi.encodePacked(keys[i])) == remark) {
                continue;
            }
            string memory keyPath = string(abi.encodePacked(".", keys[i]));
            vm.serializeAddress(
                output,
                keys[i],
                vm.parseJsonAddress(output, keyPath)
            );
        }
        return output;
    }

    function saveJson(string memory json) private {
        string memory finalJson = vm.serializeString(
            json,
            "remark",
            "Deployment"
        );
        vm.writeJson(finalJson, getOutputFilePath());
    }

    function deployVerifier() public {
        address attestation = vm.envAddress("AUTOMATA_DCAP_ATTESTATION");
        vm.startBroadcast();
        AttestationVerifier verifier = new AttestationVerifier(attestation, false);
        vm.stopBroadcast();
        string memory output = readJson();
        vm.serializeAddress(output, "AttestationVerifier", address(verifier));
        saveJson(output);
    }

    function deployProverVerifier() public {
        uint256 chainID = vm.envUint("CHAIN_ID");
        uint256 attestValiditySeconds = vm.envUint("ATTEST_VALIDITY_SECONDS");
        uint256 maxBlockNumberDiff = vm.envUint("MAX_BLOCK_NUMBER_DIFF");

        string memory output = readJson();
        address attestationAddr = vm.parseJsonAddress(
            output,
            ".AttestationVerifier"
        );

        vm.startBroadcast();
        ProverRegistry registryImpl = new ProverRegistry(address(attestationAddr), chainID, attestValiditySeconds, maxBlockNumberDiff);
        vm.stopBroadcast();

        vm.serializeAddress(output, "ProxyRegistry", address(registryImpl));
        saveJson(output);
    }

    function deployAll() public {
        deployVerifier();
        deployProverVerifier();
    }
}