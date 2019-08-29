# -------------------------------------------------------------------------------------------------
# <copyright file="order.pxd" company="Nautech Systems Pty Ltd">
#  Copyright (C) 2015-2019 Nautech Systems Pty Ltd. All rights reserved.
#  The use of this source code is governed by the license as found in the LICENSE.md file.
#  https://nautechsystems.io
# </copyright>
# -------------------------------------------------------------------------------------------------

from cpython.datetime cimport datetime

from nautilus_trader.core.types cimport GUID
from nautilus_trader.model.c_enums.order_side cimport OrderSide
from nautilus_trader.model.c_enums.order_type cimport OrderType
from nautilus_trader.model.c_enums.order_status cimport OrderStatus
from nautilus_trader.model.c_enums.time_in_force cimport TimeInForce
from nautilus_trader.common.clock cimport Clock
from nautilus_trader.model.objects cimport Quantity, Symbol, Price
from nautilus_trader.model.events cimport OrderEvent, OrderInitialized
from nautilus_trader.model.identifiers cimport Label, OrderId, AccountId, ExecutionId, ExecutionTicket
from nautilus_trader.model.generators cimport OrderIdGenerator


cdef class Order:
    """
    Represents an order in a financial market.
    """
    cdef set _order_ids_broker
    cdef set _execution_ids
    cdef set _execution_tickets
    cdef list _events

    cdef readonly OrderId id
    cdef readonly OrderId id_current
    cdef readonly OrderId id_broker
    cdef readonly AccountId account_id
    cdef readonly ExecutionId execution_id
    cdef readonly ExecutionTicket execution_ticket
    cdef readonly Symbol symbol
    cdef readonly OrderSide side
    cdef readonly OrderType type
    cdef readonly Quantity quantity
    cdef readonly datetime timestamp
    cdef readonly Price price
    cdef readonly Label label
    cdef readonly TimeInForce time_in_force
    cdef readonly datetime expire_time
    cdef readonly Quantity filled_quantity
    cdef readonly datetime filled_timestamp
    cdef readonly Price average_price
    cdef readonly object slippage
    cdef readonly OrderStatus status
    cdef readonly GUID init_id
    cdef readonly OrderEvent last_event
    cdef readonly bint is_buy
    cdef readonly bint is_sell
    cdef readonly bint is_working
    cdef readonly bint is_completed

    @staticmethod
    cdef Order create(OrderInitialized event)
    cdef bint equals(self, Order other)
    cpdef str status_as_string(self)
    cpdef list get_order_ids_broker(self)
    cpdef list get_execution_ids(self)
    cpdef list get_execution_tickets(self)
    cpdef list get_events(self)
    cpdef int event_count(self)
    cpdef void apply(self, OrderEvent event)
    cdef void _set_state_to_working(self)
    cdef void _set_state_to_completed(self)
    cdef void _set_slippage(self)
    cdef void _set_fill_status(self)


cdef class AtomicOrder:
    """
    Represents an order for a financial market instrument consisting of a 'parent'
    entry order and 'child' OCO orders being a stop-loss and optional profit target.
    """
    cdef readonly OrderId id
    cdef readonly Order entry
    cdef readonly Order stop_loss
    cdef readonly Order take_profit
    cdef readonly bint has_take_profit
    cdef readonly datetime timestamp

    cdef bint equals(self, AtomicOrder other)


cdef class OrderFactory:
    """
    Provides creation of different order types. Holds an internal OrderIdGenerator
    to handle order_ids.
    """
    cdef Clock _clock
    cdef OrderIdGenerator _id_generator

    cpdef void reset(self)

    cpdef Order market(
            self,
            Symbol symbol,
            OrderSide order_side,
            Quantity quantity,
            Label label=*)

    cpdef Order limit(
            self,
            Symbol symbol,
            OrderSide order_side,
            Quantity quantity,
            Price price,
            Label label=*,
            TimeInForce time_in_force=*,
            datetime expire_time=*)

    cpdef Order stop_market(
            self,
            Symbol symbol,
            OrderSide order_side,
            Quantity quantity,
            Price price,
            Label label=*,
            TimeInForce time_in_force=*,
            datetime expire_time=*)

    cpdef Order stop_limit(
            self,
            Symbol symbol,
            OrderSide order_side,
            Quantity quantity,
            Price price,
            Label label=*,
            TimeInForce time_in_force=*,
            datetime expire_time=*)

    cpdef Order market_if_touched(
            self,
            Symbol symbol,
            OrderSide order_side,
            Quantity quantity,
            Price price,
            Label label=*,
            TimeInForce time_in_force=*,
            datetime expire_time=*)

    cpdef Order fill_or_kill(
            self,
            Symbol symbol,
            OrderSide order_side,
            Quantity quantity,
            Label label=*)

    cpdef Order immediate_or_cancel(
            self,
            Symbol symbol,
            OrderSide order_side,
            Quantity quantity,
            Label label=*)

    cpdef AtomicOrder atomic_market(
            self,
            Symbol symbol,
            OrderSide order_side,
            Quantity quantity,
            Price price_stop_loss,
            Price price_take_profit=*,
            Label label=*)

    cpdef AtomicOrder atomic_limit(
            self,
            Symbol symbol,
            OrderSide order_side,
            Quantity quantity,
            Price price_entry,
            Price price_stop_loss,
            Price price_take_profit=*,
            Label label=*,
            TimeInForce time_in_force=*,
            datetime expire_time=*)

    cpdef AtomicOrder atomic_stop_market(
            self,
            Symbol symbol,
            OrderSide order_side,
            Quantity quantity,
            Price price_entry,
            Price price_stop_loss,
            Price price_take_profit=*,
            Label label=*,
            TimeInForce time_in_force=*,
            datetime expire_time=*)

    cdef AtomicOrder _create_atomic_order(
        self,
        Order entry,
        Price price_stop_loss,
        Price price_take_profit,
        Label original_label)
