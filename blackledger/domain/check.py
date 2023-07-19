from typing import Optional

from . import errors, model


def transaction_entries_at_least_two(tx: model.Transaction) -> Optional[errors.Error]:
    if len(tx.entries) < 2:
        return errors.NotEnoughEntries()


def transaction_entries_use_same_currency(
    tx: model.Transaction,
) -> Optional[errors.Error]:
    currencies = set(e.amount.currency for e in tx.entries)
    if len(currencies) > 1:
        return errors.UnmatchedCurrencies()


def transaction_entries_must_balance(tx: model.Transaction) -> Optional[errors.Error]:
    if tx.balance() != 0:
        return errors.OutOfBalance()
