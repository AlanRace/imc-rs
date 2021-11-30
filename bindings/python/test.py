#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Created on Mon Oct  4 15:01:47 2021

@author: alan
"""

import pyimc

import matplotlib.pyplot as plt
import time


data = pyimc.Mcd.parse_with_dcm('/home/alan/Documents/Work/IMC/set1.mcd')
data = pyimc.Mcd.parse('/home/alan/Documents/Work/IMC/set1.mcd')
slide = data.slide(1)
panorama = data.panorama(3)

start = time.time()
image = panorama.image()
end = time.time()
print(end - start)

plt.imshow(image)

#%%
acquisition = data.acquisition(data.acquisition_ids()[0])

channels = acquisition.channels()

dna_channel = channels[8]

overview_image = slide.overview_image(7500, dna_channel, 100)

plt.imsave('/home/alan/Documents/Work/IMC/overview.png', overview_image);

import time

start = time.time()
dna_data = acquisition.channel_data(dna_channel);
end = time.time()
print(end - start)

plt.imshow(dna_data)


plt.imsave('/home/alan/Documents/Work/IMC/dna_channel.png', dna_data, vmax=100);