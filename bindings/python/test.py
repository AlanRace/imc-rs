#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Created on Mon Oct  4 15:01:47 2021

@author: alan
"""

import pyimc

import matplotlib.pyplot as plt
import time


data = pyimc.MCD.parse('/home/alan/Documents/Work/IMC/set1.mcd')
panorama = data.panorama(3)

start = time.time()
image = panorama.image()
end = time.time()
print(end - start)

plt.imshow(image)