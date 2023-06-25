# -------------------------------------------------------------------------------------------------
#  Copyright (C) 2015-2023 Nautech Systems Pty Ltd. All rights reserved.
#  https://nautechsystems.io
#
#  Licensed under the GNU Lesser General Public License Version 3.0 (the "License");
#  You may not use this file except in compliance with the License.
#  You may obtain a copy of the License at https://www.gnu.org/licenses/lgpl-3.0.en.html
#
#  Unless required by applicable law or agreed to in writing, software
#  distributed under the License is distributed on an "AS IS" BASIS,
#  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
#  See the License for the specific language governing permissions and
#  limitations under the License.
# -------------------------------------------------------------------------------------------------

import json
import os

import pytest

from nautilus_trader.adapters.binance.common.enums import BinanceAccountType
from nautilus_trader.adapters.binance.factories import get_cached_binance_http_client
from nautilus_trader.adapters.binance.futures.http.wallet import BinanceFuturesWalletHttpAPI
from nautilus_trader.common.clock import LiveClock
from nautilus_trader.common.logging import Logger


@pytest.mark.asyncio()
async def test_binance_futures_testnet_wallet_http_client():
    clock = LiveClock()

    client = get_cached_binance_http_client(
        clock=clock,
        logger=Logger(clock=clock),
        account_type=BinanceAccountType.USDT_FUTURE,
        key=os.getenv("BINANCE_FUTURES_TESTNET_API_KEY"),
        secret=os.getenv("BINANCE_FUTURES_TESTNET_API_SECRET"),
        is_testnet=True,
    )

    wallet = BinanceFuturesWalletHttpAPI(clock=clock, client=client)
    response = await wallet.commission_rate(symbol="BTCUSDT")
    print(json.dumps(response, indent=4))
