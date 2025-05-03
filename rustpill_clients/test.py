# %%
import logging
from rustpill_clients import ServoClient

FORMAT = "%(levelname)s %(name)s %(asctime)-15s %(filename)s:%(lineno)d %(message)s"
logging.basicConfig(format=FORMAT)
logging.getLogger().setLevel(logging.INFO)
# %%
servo = ServoClient()
# %%
if True:  # big servo
    servo.set_servo_min(722)
    servo.set_servo_max(2306)
# %%
servo.set_angle(00)
# %%
servo.pingx2(2137)
# %%
servo.get_id()
# %%
servo.get_angle()
# %%
servo.set_angle(180)
# %%
servo.get_config()
