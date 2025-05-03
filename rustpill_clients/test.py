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
    servo.configure_channel(
        2,
        min_angle_duty_cycle=servo.us_to_duty_cycle(722),
        max_angle_duty_cycle=servo.us_to_duty_cycle(2306),
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
servo.get_config()
