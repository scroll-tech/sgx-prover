// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {AttestationVerifier} from "src/core/AttestationVerifier.sol";
import {IProverRegistry} from "src/interfaces/IProverRegistry.sol";
import "@openzeppelin/contracts/utils/cryptography/ECDSA.sol";

contract ProverRegistry is IProverRegistry {

    AttestationVerifier public verifier;
    uint256 public attestValiditySeconds;
    uint256 public maxBlockNumberDiff;
    uint256 public chainID;

    mapping(bytes32 reportHash => bool used) public attestedReports;
    mapping(address proverAddr => ProverInstance) public attestedProvers;

    uint256[43] private __gap;

    constructor(
        address _verifierAddr,
        uint256 _chainID,
        uint256 _attestValiditySeconds,
        uint256 _maxBlockNumberDiff
    ) {
        verifier = AttestationVerifier(_verifierAddr);
        chainID = _chainID;
        attestValiditySeconds = _attestValiditySeconds;
        maxBlockNumberDiff = _maxBlockNumberDiff;
    }

    /// @notice register prover instance with quote
    function register(
        bytes calldata _report,
        ReportData calldata _data
    ) external {
        _checkBlockNumber(_data.referenceBlockNumber, _data.referenceBlockHash);
        bytes32 dataHash = keccak256(abi.encode(_data));

        verifier.verifyAttestation(_report, dataHash);

        bytes32 reportHash = keccak256(_report);
        if (attestedReports[reportHash]) revert REPORT_USED();
        attestedReports[reportHash] = true;

        uint256 validUnitl = block.timestamp + attestValiditySeconds;
        attestedProvers[_data.addr] = ProverInstance(
            _data.addr,
            validUnitl,
            _data.teeType
        );
        emit InstanceAdded(_data.addr, validUnitl);
    }

    /// TODO: each proof should coming from different teeType
    /// @notice verify multiple proofs in one call
    function verifyProofs(Proof[] calldata _proofs) external view {
        require(_proofs.length >= 1, "missing proofs");
        for (uint i = 0; i < _proofs.length; i++) {
            Proof calldata proof = _proofs[i];
            address oldInstance = recoverOldInstance(proof.poe, proof.signature);

            ProverInstance memory prover = checkProver(oldInstance);
            if (proof.teeType != prover.teeType) revert PROVER_TYPE_MISMATCH();
        }
    }

    function recoverOldInstance(
        Poe memory _poe,
        bytes memory _signature
    ) public view returns (address) {
        return ECDSA.recover(
            keccak256(getSignedMsg(_poe)),
            _signature
        );
    }

    function checkProver(
        address _proverAddr
    ) public view returns (ProverInstance memory) {
        ProverInstance memory prover;
        if (_proverAddr == address(0)) revert PROVER_INVALID_ADDR(_proverAddr);
        prover = attestedProvers[_proverAddr];
        if (prover.validUntil < block.timestamp) revert PROVER_OUT_OF_DATE(prover.validUntil);
        return prover;
    }

    function getSignedMsg(
        Poe memory _poe
    ) public view returns (bytes memory) {
        return abi.encode(
            "POE",
            chainID,
            address(this),
            _poe
        );
    }

    // Due to the inherent unpredictability of blockHash, it mitigates the risk of mass-generation 
    //   of attestation reports in a short time frame, preventing their delayed and gradual exploitation.
    // This function will make sure the attestation report generated in recent ${maxBlockNumberDiff} blocks
    function _checkBlockNumber(
        uint256 blockNumber,
        bytes32 blockHash
    ) private view {
        if (blockNumber >= block.number) revert INVALID_BLOCK_NUMBER();
        if (block.number - blockNumber >= maxBlockNumberDiff)
            revert BLOCK_NUMBER_OUT_OF_DATE();
        if (blockhash(blockNumber) != blockHash) revert BLOCK_NUMBER_MISMATCH();
    }
}