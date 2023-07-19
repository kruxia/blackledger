from typing import Optional

from . import check, errors, model, types


def transaction_errors(tx: model.Transaction) -> list[errors.Error]:
    """
    * Verify a transaction:
      - it has at least two entries
      - all entries use the same currency
      - it balances: debits + credits = 0
    * Return a list of Errors that occurred -- if there are no Errors, it's verified.
    """
    errors = []
    if error := check.transaction_entries_at_least_two(tx):
        errors.append(error)
    if error := check.transaction_entries_use_same_currency(tx):
        errors.append(error)
    if error := check.transaction_entries_must_balance(tx):
        errors.append(error)

    return errors


def transaction_generate_missing_entry(
    tx: model.Transaction, use_account: model.Account
) -> Optional[model.Entry]:
    """
    - Given a transaction, if the transaction doesn't balance, generate an Entry in the
      `use_account` Account. If the transaction does balance, do nothing.

    - instead of editing the transaction, just return an Entry that can be added to the
      transaction.entries.
    """
    balance = tx.balance()
    if balance != 0:
        if abs(balance) / balance == types.Direction.DR:
            direction = types.Direction.CR
        else:
            direction = types.Direction.DR

        return model.Entry(
            account=use_account,
            amount=model.Amount(decimal=abs(balance), currency=tx.currency()),
            direction=direction,
        )
