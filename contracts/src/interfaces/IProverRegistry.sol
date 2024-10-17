// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

interface IProverRegistry {
    struct ProverInstance {
        address addr;
        uint256 validUntil;
        uint256 teeType;
    }

    struct Poe {
        bytes32 batchHash;
        bytes32 prevStateRoot;
        bytes32 newStateRoot;
        bytes32 withdrawalRoot;
    }

    struct Proof {
        Poe poe;
        bytes signature;
        uint256 teeType; // 1: IntelTDX
    }

    struct ReportData {
        address addr;
        uint256 teeType;
        uint256 referenceBlockNumber;
        bytes32 referenceBlockHash;
    }

    error INVALID_BLOCK_NUMBER();
    error BLOCK_NUMBER_OUT_OF_DATE();
    error BLOCK_NUMBER_MISMATCH();
    error REPORT_USED();
    error INVALID_PROVER_INSTANCE();
    error PROVER_TYPE_MISMATCH();
    error INVALID_REPORT();
    error INVALID_REPORT_DATA();
    error REPORT_DATA_MISMATCH();
    error PROVER_INVALID_INSTANCE_ID(uint256);
    error PROVER_INVALID_ADDR(address);
    error PROVER_ADDR_MISMATCH(address, address);
    error PROVER_OUT_OF_DATE(uint256);

    event InstanceAdded(
        address indexed id,
        uint256 validUntil
    );

    /// @notice register prover instance with quote
    function register(
        bytes calldata _report,
        ReportData calldata _data
    ) external;

    /// TODO: should we need to add teeType?
    /// @notice validate whether the prover with (instanceID, address)
    function checkProver(
        address _proverAddr
    ) external view returns (ProverInstance memory);

    /// TODO: each proof should coming from different teeType
    /// @notice verify multiple proofs in one call
    function verifyProofs(Proof[] calldata _proofs) external;
}