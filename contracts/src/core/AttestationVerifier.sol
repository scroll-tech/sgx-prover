// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {IAttestation} from "src/interfaces/IAttestation.sol";
import {BELE} from "src/utils/BELE.sol";
import {BytesUtils} from "src/utils/BytesUtils.sol";
import {Test, console} from "forge-std/Test.sol";

contract AttestationVerifier {
    using BytesUtils for bytes;
    struct EnclaveReport {
        bytes16 cpuSvn;
        bytes4 miscSelect;
        bytes28 reserved1;
        bytes16 attributes;
        bytes32 mrEnclave;
        bytes32 reserved2;
        bytes32 mrSigner;
        bytes reserved3; // 96 bytes
        uint16 isvProdId;
        uint16 isvSvn;
        bytes reserved4; // 60 bytes
        bytes reportData; // 64 bytes - For QEReports, this contains the hash of the concatenation of attestation key and QEAuthData
    }

    IAttestation public attestationVerifier;
    address public owner;
    bool immutable private checkLocalEnclaveReport;
    mapping(bytes32 enclave => bool trusted) private trustedUserMrEnclave;
    mapping(bytes32 signer => bool trusted) private trustedUserMrSigner;

    constructor(address _attestationVerifierAddr, bool _checkLocalEnclaveReport) {
        attestationVerifier = IAttestation(_attestationVerifierAddr);
        checkLocalEnclaveReport = _checkLocalEnclaveReport;
        owner = msg.sender;
    }
    
    modifier onlyOwner() {
        require(msg.sender == owner, "onlyOwner");
        _;
    }

    function setOwner(address _owner) external onlyOwner {
        owner = _owner;
    }

    function setMrSigner(bytes32 _mrSigner, bool _trusted) external onlyOwner {
        trustedUserMrSigner[_mrSigner] = _trusted;
    }

    function setMrEnclave(bytes32 _mrEnclave, bool _trusted) external onlyOwner {
        trustedUserMrEnclave[_mrEnclave] = _trusted;
    }

    error INVALID_REPORT();
    error INVALID_REPORT_DATA();
    error REPORT_DATA_MISMATCH();
    error INVALID_MR_SIGNER();
    error INVALID_MR_ENCLAVE();

    function verifyAttestation(
        bytes calldata _report,
        bytes32 _userData
    ) public {
        (bool succ, bytes memory output) = attestationVerifier.verifyAndAttestOnChain(_report);
        if (!succ) revert INVALID_REPORT();
        EnclaveReport memory report = extractEnclaveReport(output);
        bytes32 reportUserData = report.reportData.readBytes32(32);
        if (reportUserData != _userData) revert REPORT_DATA_MISMATCH();
        if (checkLocalEnclaveReport) {
            if (!trustedUserMrEnclave[report.mrEnclave]) revert INVALID_MR_ENCLAVE();
            if (!trustedUserMrSigner[report.mrSigner]) revert INVALID_MR_SIGNER();
        }
    }

    function extractEnclaveReport(bytes memory output) internal pure returns (EnclaveReport memory) {
        uint256 offset = 13;
        uint len = output.length - offset;
        (bool succ, EnclaveReport memory enclaveReport) = parseEnclaveReport(output.substring(13, len));
        if (!succ) revert INVALID_REPORT();
        return enclaveReport;
    }

    function parseEnclaveReport(bytes memory rawEnclaveReport)
        internal
        pure
        returns (bool success, EnclaveReport memory enclaveReport)
    {
        if (rawEnclaveReport.length != 384) {
            return (false, enclaveReport);
        }
        enclaveReport.cpuSvn = bytes16(rawEnclaveReport.substring(0, 16));
        enclaveReport.miscSelect = bytes4(rawEnclaveReport.substring(16, 4));
        enclaveReport.reserved1 = bytes28(rawEnclaveReport.substring(20, 28));
        enclaveReport.attributes = bytes16(rawEnclaveReport.substring(48, 16));
        enclaveReport.mrEnclave = bytes32(rawEnclaveReport.substring(64, 32));
        enclaveReport.reserved2 = bytes32(rawEnclaveReport.substring(96, 32));
        enclaveReport.mrSigner = bytes32(rawEnclaveReport.substring(128, 32));
        enclaveReport.reserved3 = rawEnclaveReport.substring(160, 96);
        enclaveReport.isvProdId = uint16(BELE.leBytesToBeUint(rawEnclaveReport.substring(256, 2)));
        enclaveReport.isvSvn = uint16(BELE.leBytesToBeUint(rawEnclaveReport.substring(258, 2)));
        enclaveReport.reserved4 = rawEnclaveReport.substring(260, 60);
        enclaveReport.reportData = rawEnclaveReport.substring(320, 64);
        success = true;
    }
}