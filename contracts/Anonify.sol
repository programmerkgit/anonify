pragma solidity ^0.5.0;
pragma experimental ABIEncoderV2;

import "./ReportHandle.sol";
import "./utils/Secp256k1.sol";
import "./utils/BytesUtils.sol";

// Consider: Avoid inheritting
contract Anonify is ReportHandle {
    address private _owner;
    uint32 private _mrenclaveVer;

    // An counter of registered roster index
    uint32 private _rosterIdxCounter;
    // Mapping of a sender and roster index
    mapping(address => uint32) private _senderToRosterIdx;

    event StoreCiphertext(bytes ciphertext);
    event StoreHandshake(bytes handshake);
    event UpdateMrenclaveVer(uint32 newVersion);

    constructor(
        bytes memory _report,
        bytes memory _reportSig,
        bytes memory _handshake,
        uint32 mrenclaveVer
    ) public ReportHandle(_report, _reportSig) {
        // The offset of roster index is 4.
        uint32 rosterIdx = BytesUtils.toUint32(_handshake, 4);
        require(rosterIdx == 0, "First roster_idx must be zero");

        _owner = msg.sender;
        _mrenclaveVer = mrenclaveVer;
        _senderToRosterIdx[msg.sender] = rosterIdx;
        _rosterIdxCounter = rosterIdx;
        handshake_wo_sig(_handshake);
    }

    modifier onlyOwner() {
        require(_owner == msg.sender, "caller is not the owner");
        _;
    }

    // a new TEE node joins the group.
    function joinGroup(
        bytes memory _report,
        bytes memory _reportSig,
        bytes memory _handshake,
        uint32 _version,
        uint32 _rosterIdx
    ) public {
        require(_mrenclaveVer == _version, "Must be same version");
        require(
            _rosterIdx == _rosterIdxCounter + 1,
            "Joining the group must be ordered accordingly by roster index"
        );
        require(
            _senderToRosterIdx[msg.sender] == 0,
            "The msg.sender can join only once"
        );

        handleReport(_report, _reportSig);
        _senderToRosterIdx[msg.sender] = _rosterIdx;
        _rosterIdxCounter = _rosterIdx;
        handshake_wo_sig(_handshake);
    }

    // a recovered TEE node registers the report
    function registerReport(
        bytes memory _report,
        bytes memory _reportSig,
        uint32 _version,
        uint32 _rosterIdx
    ) public {
        require(_mrenclaveVer == _version, "Must be same version");

        handleReport(_report, _reportSig);
        _senderToRosterIdx[msg.sender] = _rosterIdx;
    }

    function updateMrenclave(
        bytes memory _report,
        bytes memory _reportSig,
        bytes memory _handshake,
        uint32 _newVersion,
        uint32 _rosterIdx
    ) public onlyOwner {
        require(_mrenclaveVer != _newVersion, "Must be new version");
        require(_rosterIdx == 0, "Only owner can update mrenclave");

        updateMrenclaveInner(_report, _reportSig);
        handshake_wo_sig(_handshake);
        _mrenclaveVer = _newVersion;
        emit UpdateMrenclaveVer(_newVersion);
    }

    // Store ciphertexts which is generated by trusted environment.
    function storeCommand(bytes memory _newCiphertext, bytes memory _enclaveSig)
        public
    {
        address verifyingKey = Secp256k1.recover(
            sha256(_newCiphertext),
            _enclaveSig
        );
        require(
            verifyingKeyMapping[verifyingKey] == verifyingKey,
            "Invalid enclave signature."
        );

        emit StoreCiphertext(_newCiphertext);
    }

    function handshake(
        bytes memory _handshake,
        bytes memory _enclaveSig,
        uint32 _rosterIdx
    ) public {
        require(
            _senderToRosterIdx[msg.sender] == _rosterIdx,
            "The roster index must be same as the registered one"
        );
        address verifyingKey = Secp256k1.recover(
            sha256(abi.encodePacked(_handshake, _rosterIdx)),
            _enclaveSig
        );
        require(
            verifyingKeyMapping[verifyingKey] == verifyingKey,
            "Invalid enclave signature."
        );

        emit StoreHandshake(_handshake);
    }

    function handshake_wo_sig(bytes memory _handshake) private {
        emit StoreHandshake(_handshake);
    }
}
