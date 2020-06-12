pragma solidity ^0.5.0;
pragma experimental ABIEncoderV2;

import "./ReportHandle.sol";
import "./utils/Secp256k1.sol";

// Consider: Avoid inheritting
contract Anonify is ReportHandle {
    event StoreCiphertext(bytes ciphertext);
    event StoreHandshake(bytes handshake);

    // Encrypted states
    // mapping(uint256 => bytes[]) private _ciphertexts;
    // Store lock parameters to avoid data collision.
    // mapping(uint256 => mapping (bytes32 => bytes32)) private _lockParams;

    constructor(
        bytes memory _report,
        bytes memory _reportSig,
        bytes memory _handshake
    ) ReportHandle(_report, _reportSig) public {
        handshake(_handshake);
     }

    // Register a new TEE participant.
    function register(
        bytes memory _report,
        bytes memory _reportSig,
        bytes memory _handshake
    ) public {
        handleReport(_report, _reportSig);
        handshake(_handshake);
    }

    // Store ciphertexts which is generated by trusted environment.
    function stateTransition(
        uint256 _stateId,
        bytes memory _newCiphertext,
        bytes memory _enclaveSig,
        bytes32 _msg
    ) public {
        // uint256 param_len = _newLockParams.length;
        // require(param_len == _newCiphertexts.length, "Invalid parameter length.");

        address inpEnclaveAddr = Secp256k1.recover(_msg, _enclaveSig);
        require(enclaveAddress[inpEnclaveAddr] == inpEnclaveAddr, "Invalid enclave signature.");

        emit StoreCiphertext(_newCiphertext);

        // for (uint32 i = 0; i < param_len; i++) {
        //     require(_lockParams[_stateId][_newLockParams[i]] == 0, "The state has already been modified.");

        //      _lockParams[_stateId][_newLockParams[i]] = _newLockParams[i];
        //     _ciphertexts[_stateId].push(_newCiphertexts[i]);

        //     // Emit event over iterations because ABIEncoderV2 is not supported by web3-rust.
        //     emit StoreCiphertext(_newCiphertexts[i]);
        // }
    }

    function handshake(bytes memory _handshake) public {
        emit StoreHandshake(_handshake);
    }
}
