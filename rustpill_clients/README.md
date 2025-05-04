# Rustpill Firmware Python Bindings

This package provides Python bindings for performing Remote Procedure Calls (RPC) with the rustpill firmware.

## Getting Started

### Prerequisites
- Python 3.9 or later.
- rustpill firmware deployed on your target device.
- USB connection

## Usage

Below is a simple example to perform an RPC call using the bindings:

```python
# %%
import logging
from rustpill_clients import ServoClient

FORMAT = "%(levelname)s %(name)s %(asctime)-15s %(filename)s:%(lineno)d %(message)s"
logging.basicConfig(format=FORMAT)
logging.getLogger().setLevel(logging.INFO)
# %%
servo = ServoClient()
# %%
servo.configure_channel(
    2,
    min_angle_duty_cycle=servo.us_to_duty_cycle(500),
    max_angle_duty_cycle=servo.us_to_duty_cycle(2500),
)
# %%
servo.set_angle(2, 0)
# %%
servo.pingx2(2137)
# %%
servo.get_id()
# %%
servo.get_angle(2)
# %%
servo.set_angle(2, 180)
# %%
servo.config

```
