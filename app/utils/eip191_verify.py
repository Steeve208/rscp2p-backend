"""
Verificación EIP-191 (personal_sign) sin importar eth_account.Account.

En algunos Windows, Smart App Control / WDAC bloquea el DLL nativo de
`bitarray`, que se carga al importar `eth_account.account`. El submódulo
`eth_account.messages` no arrastra bitarray; la recuperación de clave usa
`eth_keys` con backend nativo (Python) si se fuerza antes del primer uso.
"""

from __future__ import annotations

import os

os.environ.setdefault("ECC_BACKEND_CLASS", "eth_keys.backends.NativeECCBackend")

from eth_account.messages import encode_defunct, _hash_eip191_message
from eth_keys import keys
from eth_utils import to_bytes, to_int
from hexbytes import HexBytes

CHAIN_ID_OFFSET = 35
V_OFFSET = 27


def _extract_chain_id(raw_v: int) -> tuple[int | None, int]:
    above_id_offset = raw_v - CHAIN_ID_OFFSET
    if above_id_offset < 0:
        if raw_v in {0, 1}:
            return (None, raw_v + V_OFFSET)
        if raw_v in {27, 28}:
            return (None, raw_v)
        raise ValueError(f"v {raw_v!r} is invalid, must be one of: 0, 1, 27, 28, 35+")
    chain_id, v_bit = divmod(above_id_offset, 2)
    return (chain_id, v_bit + V_OFFSET)


def _to_standard_v(enhanced_v: int) -> int:
    _chain, chain_naive_v = _extract_chain_id(enhanced_v)
    v_standard = chain_naive_v - V_OFFSET
    if v_standard not in (0, 1):
        raise ValueError("invalid v after normalization")
    return v_standard


def _to_standard_signature_bytes(ethereum_signature_bytes: bytes) -> bytes:
    rs = ethereum_signature_bytes[:-1]
    v = to_int(ethereum_signature_bytes[-1])
    standard_v = _to_standard_v(v)
    return rs + to_bytes(standard_v)


def recover_checksum_address(*, text_message: str, signature: str) -> str:
    """Devuelve la dirección checksummada que firmó `text_message` con personal_sign."""
    signable = encode_defunct(text=text_message)
    message_hash = _hash_eip191_message(signable)
    hash_bytes = HexBytes(message_hash)
    if len(hash_bytes) != 32:
        raise ValueError("message hash must be 32 bytes")
    signature_bytes = HexBytes(signature)
    signature_bytes_standard = _to_standard_signature_bytes(bytes(signature_bytes))
    signature_obj = keys.Signature(signature_bytes=signature_bytes_standard)
    pubkey = signature_obj.recover_public_key_from_msg_hash(hash_bytes)
    return pubkey.to_checksum_address()
