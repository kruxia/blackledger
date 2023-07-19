from uuid import UUID

import pytest

from blackledger.domain import types


def test_id_generate():
    """
    Calling the ID() initializer generates a new ID instance.
    """
    i = types.ID()
    assert isinstance(i, types.ID)
    assert isinstance(i.bytes, bytes)
    assert isinstance(i.hex, str)


def test_id_instantiate():
    """
    Calling the ID() initializer with a base58-encoded ULID/UUID instantiates that ID.
    """
    i1 = types.ID()
    s = str(i1)
    i2 = types.ID(s)
    assert i2 == i1


def test_id_hashable():
    """
    ID is hashable, so it can be used as a dict key.
    """
    i = types.ID()
    assert isinstance(hash(i), int)
    d = {i: 1}
    d[i] = 2


def test_id_immutable():
    """
    ID is immutable, so a dict key will not disappear.
    """
    i = types.ID()
    b = types.ID().bytes
    with pytest.raises(TypeError):
        i.bytes = b


def test_id_uuid():
    """
    ID().uuid is a UUID instance
    """
    i = types.ID()
    assert isinstance(i.uuid, UUID)


def test_currency_instantiate_valid():
    """
    Valid currencies are uppercase ASCII letters.
    """
    c = types.Currency("USD")
    assert isinstance(c, types.Currency)
    assert str(c) == "USD"


@pytest.mark.parametrize("c", ["usd", "C8", "US-D", "USd", "$"])
def test_currency_instantiate_invalid(c):
    """
    Currencies that are not only uppercase ASCII letters will not instantiate.
    """
    with pytest.raises(ValueError):
        types.Currency(c)


def test_currency_repr():
    c = types.Currency("USD")
    assert c.__class__.__name__ in repr(c)
    assert str(c) in repr(c)
