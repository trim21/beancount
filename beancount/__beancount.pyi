import datetime
from decimal import Decimal
from typing import Optional, Dict, Any, FrozenSet

class File:
    includes: list[str]
    options: list[str]
    directives: list[Any]

def parse(b: str) -> File: ...

Flag = str
Account = str
Currency = str
Meta = Dict[str, Any]

class Open:
    """
    An "open account" directive.

    Attributes:
      meta: See above.
      date: See above.
      account: A string, the name of the account that is being opened.
      currencies: A list of strings, currencies that are allowed in this account.
        May be None, in which case it means that there are no restrictions on which
        currencies may be stored in this account.
      booking: A Booking enum, the booking method to use to disambiguate
        postings to this account (when zero or more than one postings match the
        specification), or None if not specified. In practice, this attribute will
        be should be left unspecified (None) in the vast majority of cases. See
        Booking below for a selection of valid methods.
    """

    meta: Meta
    date: datetime.date
    account: Account
    currencies: list[Currency]
    booking: Optional[Booking]

class Close:
    """
    A "close account" directive.

    Attributes:
      meta: See above.
      date: See above.
      account: A string, the name of the account that is being closed.
    """

    meta: Meta
    date: datetime.date
    account: Account

class Commodity:
    """
    An optional commodity declaration directive. Commodities generally do not need
    to be declared, but they may, and this is mainly created as intended to be
    used to attach meta-data on a commodity name. Whenever a plugin needs
    per-commodity meta-data, you would define such a commodity directive. Another
    use is to define a commodity that isn't otherwise (yet) used anywhere in an
    input file. (At the moment the date is meaningless but is specified for
    coherence with all the other directives; if you can think of a good use case,
    let us know).

    Attributes:
      meta: See above.
      date: See above.
      currency: A string, the commodity under consideration.
    """

    meta: Meta
    date: datetime.date
    currency: Currency

class Balance:
    """
    A "check the balance of this account" directive. This directive asserts that
    the declared account should have a known number of units of a particular
    currency at the beginning of its date. This is essentially an assertion, and
    corresponds to the final "Statement Balance" line of a real-world statement.
    These assertions act as checkpoints to help ensure that you have entered your
    transactions correctly.

    Attributes:
      meta: See above.
      date: See above.
      account: A string, the account whose balance to check at the given date.
      amount: An Amount, the number of units of the given currency you're
        expecting 'account' to have at this date.
      diff_amount: None if the balance check succeeds. This value is set to
        an Amount instance if the balance fails, the amount of the difference.
      tolerance: A Decimal object, the amount of tolerance to use in the
        verification.
    """

    meta: Meta
    date: datetime.date
    account: Account
    amount: Amount
    tolerance: Optional[Decimal]
    diff_amount: Optional[Amount]

class CostSpec:
    number_per: Optional[Decimal]
    number_total: Optional[Decimal]
    currency: Optional[str]
    date: Optional[datetime.date]
    label: Optional[str]
    merge: Optional[bool]

class Cost:
    number: Decimal
    currency: str
    date: datetime.date
    label: Optional[str]

class Posting:
    """
    Postings are contained in Transaction entries. These represent the individual
    legs of a transaction. Note: a posting may only appear within a single entry
    (multiple transactions may not share a Posting instance), and that's what the
    entry field should be set to.

    Attributes:
      account: A string, the account that is modified by this posting.
      units: An Amount, the units of the position, or None if it is to be
        inferred from the other postings in the transaction.
      cost: A Cost or CostSpec instances, the units of the position.
      price: An Amount, the price at which the position took place, or
        None, where not relevant. Providing a price member to a posting
        automatically adds a price in the prices database at the date of the
        transaction.
      flag: An optional flag, a one-character string or None, which is to be
        associated with the posting. Most postings don't have a flag, but it can
        be convenient to mark a particular posting as problematic or pending to
        be reconciled for a future import of its account.
      meta: A dict of strings to values, the metadata that was attached
        specifically to that posting, or None, if not provided. In practice, most
        of the instances will be unlikely to have metadata.
    """

    meta: Optional[Meta]
    account: Account
    units: Optional[Amount]
    cost: Optional[Cost | CostSpec]
    price: Optional[Amount]
    flag: Optional[Flag]

# A set of valid booking method names for positions on accounts.
# See http://furius.ca/beancount/doc/inventories for a full explanation.
class Booking:
    # Reject ambiguous matches with an error.
    STRICT = "STRICT"

    # Strict booking method, but disambiguate further with sizes. Reject
    # ambiguous matches with an error but if a lot matches the size exactly,
    # accept it the oldest.
    STRICT_WITH_SIZE = "STRICT_WITH_SIZE"

    # Disable matching and accept the creation of mixed inventories.
    NONE = "NONE"

    # Average cost booking: merge all matching lots before and after.
    AVERAGE = "AVERAGE"

    # First-in first-out in the case of ambiguity.
    FIFO = "FIFO"

    # Last-in first-out in the case of ambiguity.
    LIFO = "LIFO"

    # Highest-in first-out in the case of ambiguity.
    HIFO = "HIFO"

class Amount:
    number: Optional[Decimal]
    currency: str

    def __init__(self, number: Optional[str | int | float], currency: str) -> None: ...

class Price:
    """
    A price declaration directive. This establishes the price of a currency in
    terms of another currency as of the directive's date. A history of the prices
    for each currency pairs is built and can be queried within the bookkeeping
    system. Note that because Beancount does not store any data at time-of-day
    resolution, it makes no sense to have multiple price directives at the same
    date. (Beancount will not attempt to solve this problem; this is beyond the
    general scope of double-entry bookkeeping and if you need to build a day
    trading system, you should probably use something else).

    Attributes:
      meta: See above.
      date: See above.
     currency: A string, the currency that is being priced, e.g. HOOL.
     amount: An instance of Amount, the number of units and currency that
       'currency' is worth, for instance 1200.12 USD.
    """

    meta: Meta
    date: datetime.date
    currency: Currency
    amount: Amount

    def __init__(
        self,
        meta: Meta,
        date: datetime.date,
        currency: Currency,
        amount: Amount,
    ): ...

class Pad:
    """
    A "pad this account with this other account" directive. This directive
    automatically inserts transactions that will make the next chronological
    balance directive succeeds. It can be used to fill in missing date ranges of
    transactions, as a convenience. You don't have to use this, it's sugar coating
    in case you need it, while you're entering past history into your Ledger.

    Attributes:
      meta: See above.
      date: See above.
      account: A string, the name of the account which needs to be filled.
      source_account: A string, the name of the account which is used to debit from
        in order to fill 'account'.
    """

    meta: Meta
    date: datetime.date
    account: Account
    source_account: Account

    def __init__(
        self,
        meta: Meta,
        date: datetime.date,
        account: Account,
        source_account: Account,
    ): ...

class PostingPrice:
    unit: Amount
    total: Amount

class Transaction:
    """
    A transaction! This is the main type of object that we manipulate, and the
    entire reason this whole project exists in the first place, because
    representing these types of structures with a spreadsheet is difficult.

    Attributes:
      meta: See above.
      date: See above.
      flag: A single-character string or None. This user-specified string
        represents some custom/user-defined state of the transaction. You can use
        this for various purposes. Otherwise common, pre-defined flags are defined
        under beancount.core.flags, to flags transactions that are automatically
        generated.
      payee: A free-form string that identifies the payee, or None, if absent.
      narration: A free-form string that provides a description for the transaction.
        All transactions have at least a narration string, this is never None.
      tags: A set of tag strings (without the '#'), or EMPTY_SET.
      links: A set of link strings (without the '^'), or EMPTY_SET.
      postings: A list of Posting instances, the legs of this transaction. See the
        doc under Posting above.
    """

    meta: Meta
    date: datetime.date
    flag: Flag
    payee: Optional[str]
    narration: str
    tags: FrozenSet
    links: FrozenSet
    postings: list[Posting]
