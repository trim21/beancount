import enum

from beancount.__beancount import Booking

assert Booking.STRICT == Booking.STRICT, "a == a"
assert Booking.STRICT == "STRICT", 'a == "a"'
assert Booking.STRICT is Booking.STRICT, "a is a"

# print(Price({}, datetime.date(1, 2, 3), '1', Amount('2', '3')).meta)
