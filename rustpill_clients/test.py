# %%
import logging
from rustpill_clients import ServoClient

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
# %%
