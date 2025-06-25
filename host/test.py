# %%
from rustpill_clients import Client

def pv_print(pv):
    pvs_dict = {attr: getattr(pvs, attr) for attr in pvs.__dir__() if not attr.startswith('__')}
    print(pvs_dict)

heater = Client()
#%%
heater.enable_heater()
#%%
heater.set_heater_duty(500)
#%%
heater.disable_heater()
#%%
pvs = heater.get_pid_vals()
pv_print(pvs)
#%%
kp = 0.15
ki = 0
heater.set_pid_consts(kp,ki)
#%%
heater.enable_heater()
#%%
heater.recalc_pi()
#%%
pvs = heater.get_pid_vals()
pv_print(pvs)
#%%
heater.set_setpoint(3000)
#%%
#TODO is_on for enabling PI recalculation 