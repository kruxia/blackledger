from blackledger.domain import errors

accounts = {
    "asset": {"name": "asset", "normal": "CR", "currency": "USD"},
    "liability": {"name": "liability", "normal": "DR", "currency": "USD"},
    "revenue": {"name": "revenue", "normal": "DR", "currency": "USD"},
    "expense": {"name": "expense", "normal": "CR", "currency": "USD"},
    "equity": {"name": "equity", "normal": "DR", "currency": "USD"},
}
invalid_transaction_entries = [
    # only one entry, not balanced
    (
        {errors.NotEnoughEntries(), errors.OutOfBalance()},
        [
            {
                "account": accounts["expense"],
                "amount": {"decimal": "3.50", "currency": "USD"},
                "direction": "CR",
            }
        ],
    ),
    # two entries, not balanced
    (
        {errors.OutOfBalance()},
        [
            {
                "account": accounts["expense"],
                "amount": {"decimal": "3.50", "currency": "USD"},
                "direction": "CR",
            },
            {
                "account": accounts["liability"],
                "amount": {"decimal": "3.50", "currency": "USD"},
                "direction": "CR",
            },
        ],
    ),
    # two entries, balanced, not same currency
    (
        {errors.UnmatchedCurrencies()},
        [
            {
                "account": accounts["expense"],
                "amount": {"decimal": "3.50", "currency": "CAD"},
                "direction": "CR",
            },
            {
                "account": accounts["asset"],
                "amount": {"decimal": "3.50", "currency": "USD"},
                "direction": "DR",
            },
        ],
    ),
]
valid_transaction_entries = [
    # two balanced entries
    [
        {
            "account": accounts["expense"],
            "amount": {"decimal": "3.50", "currency": "USD"},
            "direction": "CR",
        },
        {
            "account": accounts["asset"],
            "amount": {"decimal": "3.50", "currency": "USD"},
            "direction": "DR",
        },
    ],
    # three balanced entries
    [
        {
            "account": accounts["expense"],
            "amount": {"decimal": "1.50", "currency": "USD"},
            "direction": "CR",
        },
        {
            "account": accounts["expense"],
            "amount": {"decimal": "2.00", "currency": "USD"},
            "direction": "CR",
        },
        {
            "account": accounts["asset"],
            "amount": {"decimal": "3.50", "currency": "USD"},
            "direction": "DR",
        },
    ],
]
