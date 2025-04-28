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

# Optional if you use logs
FORMAT = '%(levelname)s %(name)s %(asctime)-15s %(filename)s:%(lineno)d %(message)s'
logging.basicConfig(format=FORMAT)
logging.getLogger().setLevel(logging.INFO)
# %%
servo = ServoClient()
# %%
servo.set_angle(90)
# %%
servo.pingx2(2137)
# %%
servo.get_id()
# %%
servo.get_angle()
# %%
servo.set_angle(0)
```
