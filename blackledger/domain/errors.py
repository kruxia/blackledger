class Error(Exception):
    ...


class NotEnoughEntries(Error):
    args = ["transaction must have at least 2 entries"]


class UnmatchedCurrencies(Error):
    args = ["transaction entries must all use the same currency"]


class OutOfBalance(Error):
    args = ["transaction entries must have balance==0"]
