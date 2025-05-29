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
from rustpill_clients import ServoClient
# %%
servo = ServoClient()
# %% Default config for each channel is 500us and 2500us at 50Hz, typical for 9g servo
# servo.configure_channel(
#     2,
#     min_angle_duty_cycle=servo.us_to_duty_cycle(500),
#     max_angle_duty_cycle=servo.us_to_duty_cycle(2500),
# )
# %%
servo.set_angle(2, 0)
# %% In case you need multiple bluepills, you can pass the serial number to the constructor
servo.get_serial_number()
# %%
servo.get_angle(2)
# %%
servo.set_angle(2, 180)
# %%
servo.config

```
