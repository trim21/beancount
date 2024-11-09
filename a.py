from beancount.__beancount import Booking, parse2

assert Booking.STRICT == Booking.STRICT, "a == a"
assert Booking.STRICT == "STRICT", 'a == "a"'
assert Booking.STRICT is Booking.STRICT, "a is a"

"""
include "a.bean"

option "title" "Ed’s Personal Ledger"

plugin "a.b.c" "cfg"

2020-02-01 open Assets:Bank:Test USTC
"""

file = parse2(
    """

2014-07-09 custom "budget" "..." 1 2 3 4
"""
)

(
    """


option "title" "Ed’s Personal Ledger"

2014-05-01 open Liabilities:CreditCard:CapitalOne     USD

2020-02-01 note Liabilities:CreditCard:CapitalOne "你好"

2020-02-01 open Assets:Bank:Test USTC

2014-05-05 txn "Cafe Mogador" "Lamb tagine with wine"
  Liabilities:CreditCard:CapitalOne         -37.45 USD
  Expenses:Restaurant

2014-06-01 pad Assets:BofA:Checking Equity:Opening-Balances

"""
)
print("includes", file.includes)  # ['a.bean']
#
# print(file.options)  # [Option(name="title", value="Ed’s Personal Ledger"]
# # print(file.directives)  # [Option(name="title", value="Ed’s Personal Ledger"]
#
for d in file.directives:
    print("---")
    print(d)
