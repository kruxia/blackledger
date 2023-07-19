import pytest

from blackledger.domain import actions, model
from blackledger.tests import parameters


@pytest.mark.parametrize("errors, entries", parameters.invalid_transaction_entries)
def test_transaction_errors_invalid(errors, entries):
    """
    Each of the invalid transaction entries sets produce the expected errors
    """
    tx = model.Transaction(entries=entries)
    tx_errors = actions.transaction_errors(tx)
    assert all(type(e) for e in tx_errors) == all(type(e) for e in errors)


@pytest.mark.parametrize("entries", parameters.valid_transaction_entries)
def test_transaction_errors_valid(entries):
    """
    Each of the valid transaction entries sets results in no errors.
    """
    tx = model.Transaction(entries=entries)
    tx_errors = actions.transaction_errors(tx)
    assert not tx_errors


@pytest.mark.parametrize("entries", parameters.valid_transaction_entries)
def test_transaction_generate_missing_entry(entries):
    """
    If one entry is removed from a valid entry set, transaction_generate_missing_entry
    will generate an entry that matches the removed entry.
    """
    for i in range(len(entries)):
        removed_entry = model.Entry(**entries[i])
        unbalanced_entries = entries[:i] + entries[i + 1 :]
        tx = model.Transaction(entries=unbalanced_entries)
        generated_entry = actions.transaction_generate_missing_entry(
            tx, removed_entry.account
        )
        assert generated_entry == removed_entry

        tx.entries.append(generated_entry)
        errors = actions.transaction_errors(tx)
        assert not errors


@pytest.mark.parametrize("entries", parameters.valid_transaction_entries)
def test_transaction_generate_missing_entry_noop(entries):
    """
    If the transaction has a balanced entry set, transaction_generate_missing_entry
    generates no entry.
    """
    tx = model.Transaction(entries=entries)
    generated_entry = actions.transaction_generate_missing_entry(
        tx, tx.entries[0].account
    )
    assert generated_entry is None
