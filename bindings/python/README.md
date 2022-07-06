# pyimc

Library for accessing imaging mass cytometry (IMC) data stored in .mcd files. Access is provided to all channel data, metadata and optical images stored within the file. Additionally, it is possible to generate `slide overview images` which can be used in whole slide imaging registration workflows.


## Installation
```
pip install pyimc
```

## Usage
IMC data in *.mcd files are stored in a spectrum-wise manner, in the order acquired on the instrument. This allows fast access to individual pixel 
information, but requires reading in all data from a single acquisition to generate a single channel image. To provide fast access to image data, an optional
means of opening the data is demonstrated below, with the caveat that this generates a temporary binary file in the same location as the .mcd file, the first time
this function is called, which can take a few seconds. The temporary binary file is approximately 33% as big as the original .mcd file.


**With fast access to images**

```python
import pyimc

data = pyimc.Mcd.parse_with_dcm("/path/to/data.mcd")
```

**Without fast access to images**

```python
import pyimc

data = pyimc.Mcd.parse_with_dcm("/path/to/data.mcd")

```

### Access to channel data

```python
# Get the first slide (there is usually only one)
slide = data.slide(1)

# Get list of all acquisition IDs in the data
acquisition_ids = data.acquisition_ids() 

# Get 3rd acquisition
acquisition = data.acquisition(acquisition_ids[2])

# Get the channel list for the current acquisition
channels = acquisition.channels()

# Select for 10th channel
channel = channels[9]

print(channel.label())
print(channel.name())

# Get the image data for the channel as a numpy array from the chosen acquisition
channel_data = acquisition.channel_data(channel)
```

### Access panorama image

```python
# Get panorama with ID = 3
panorama = data.panorama(3)

# Get optical image associated with the panorama
image = panorama.image()
```

### Generate `slide overview image`
```python
# Get the first slide (there is usually only one)
slide = data.slide(1)

# Get all channels in the slide
channels = slide.channels()

# Select the 10th channel
channel = channels[9]

# Generate an overview image of the slide with a width of 7500 pixels (height will be 
# automatically scaled), displaying the selected channel image in the relative location
# on the slide where the acquisition was performed, thresholding the intensity at 10
overview_image = slide.overview_image(7500, channel, 10)
```

### Access XML 
```python
xml = data.xml()
```