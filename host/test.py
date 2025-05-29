# %%
from rustpill_clients import ServoClient

servo = ServoClient()
# %%
if False:  # big servo
    servo.configure_channel(
        2,
        min_angle_duty_cycle=servo.us_to_duty_cycle(722),
        max_angle_duty_cycle=servo.us_to_duty_cycle(2306),
    )
# %%
servo.set_angle(2, 0)
# %%
servo.set_angle(2, 180)
# %%
servo.get_serial_number()
# %%
servo.get_angle(2)
# %%
servo.config

# %%
