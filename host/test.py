# %%
from rustpill_clients import Client

# ServoClient.flash()
adc = Client()
#%%
adc.get_adc_val()
#%%
adc.get_serial_number()