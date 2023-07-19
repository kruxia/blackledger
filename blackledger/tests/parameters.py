from blackledger.domain import errors, types

accounts = {
    "asset": {"name": "asset", "normal": types.Normal.CR},
    "liability": {"name": "liability", "normal": types.Normal.DR},
    "revenue": {"name": "revenue", "normal": types.Normal.DR},
    "expense": {"name": "expense", "normal": types.Normal.CR},
    "equity": {"name": "equity", "normal": types.Normal.DR},
}
invalid_transaction_entries = [
    # only one entry, not balanced
    (
        {errors.NotEnoughEntries(), errors.OutOfBalance()},
        [
            {
                "account": accounts["expense"],
                "amount": {"decimal": "3.50", "currency": "USD"},
                "direction": types.Direction.CR,
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
                "direction": types.Direction.CR,
            },
            {
                "account": accounts["liability"],
                "amount": {"decimal": 3.50, "currency": "USD"},
                "direction": types.Direction.CR,
            },
        ],
    ),
    # two entries, balanced, not same currency
    (
        {errors.UnmatchedCurrencies()},
        [
            {
                "account": accounts["expense"],
                "amount": {"decimal": 3.50, "currency": "CAD"},
                "direction": types.Direction.CR,
            },
            {
                "account": accounts["asset"],
                "amount": {"decimal": 3.50, "currency": "USD"},
                "direction": types.Direction.DR,
            },
        ],
    ),
]
valid_transaction_entries = [
    # two balanced entries
    [
        {
            "account": accounts["expense"],
            "amount": {"decimal": 3.50, "currency": "USD"},
            "direction": types.Direction.CR,
        },
        {
            "account": accounts["asset"],
            "amount": {"decimal": 3.50, "currency": "USD"},
            "direction": types.Direction.DR,
        },
    ],
    # three balanced entries
    [
        {
            "account": accounts["expense"],
            "amount": {"decimal": 1.50, "currency": "USD"},
            "direction": types.Direction.CR,
        },
        {
            "account": accounts["expense"],
            "amount": {"decimal": 2.00, "currency": "USD"},
            "direction": types.Direction.CR,
        },
        {
            "account": accounts["asset"],
            "amount": {"decimal": 3.50, "currency": "USD"},
            "direction": types.Direction.DR,
        },
    ],
]
