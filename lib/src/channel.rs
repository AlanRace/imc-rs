/// ChannelIdentifier describes how a channel can be identified
#[derive(Debug, Clone)]
pub enum ChannelIdentifier {
    // Unique identifier for the channel
    //Id(u16),
    /// Order number of the channel (order of acquisition)
    Order(i16),
    /// Name of the channel
    Name(String),
    /// Label given to the channel
    Label(String),
}

/// AcquisitionChannel represents a single channel acquired, forming part of an acquisition
#[derive(Debug)]
pub struct AcquisitionChannel {
    id: u16,
    channel_name: String,
    order_number: i16,
    acquisition_id: u16,
    channel_label: String,
}

impl AcquisitionChannel {
    pub(crate) fn new(
        id: u16,
        acquisition_id: u16,
        order_number: i16,
        name: &str,
        label: &str,
    ) -> Self {
        AcquisitionChannel {
            id,
            acquisition_id,
            order_number,
            channel_name: name.to_string(),
            channel_label: label.to_string(),
        }
    }

    /// Returns whether the specified channel identifier matches this channel
    pub fn is(&self, identifier: &ChannelIdentifier) -> bool {
        match identifier {
            ChannelIdentifier::Order(order) => {
                if self.order_number() == *order {
                    return true;
                }
            }
            ChannelIdentifier::Name(name) => {
                if self.name() == name {
                    return true;
                }
            }
            ChannelIdentifier::Label(label) => {
                if self.label() == label {
                    return true;
                }
            }
        }

        false
    }

    /// Returns the ID associated with the channel
    #[inline]
    pub fn id(&self) -> u16 {
        self.id
    }

    /// Returns the acquisition ID to which this channel belongs
    #[inline]
    pub fn acquisition_id(&self) -> u16 {
        self.acquisition_id
    }

    /// Returns the given name of the channel
    #[inline]
    pub fn name(&self) -> &str {
        &self.channel_name
    }

    /// Returns the position in the order in which this channel was acquired
    #[inline]
    pub fn order_number(&self) -> i16 {
        self.order_number
    }

    /// Returns the label given to the channel
    #[inline]
    pub fn label(&self) -> &str {
        &self.channel_label
    }
}
