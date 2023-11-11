class Error(Exception):
    msg = "An error occurred."

    def __init__(self, *args, **kwargs):
        super().__init__(*([self.msg] + list(args)), **kwargs)


class NotEnoughEntries(Error):
    msg = "Transaction must have at least 2 entries."


class UnmatchedCurrencies(Error):
    msg = "Transaction entries must all use the same currency."


class OutOfBalance(Error):
    msg = "Transaction entries must have balance==0."
