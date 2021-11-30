pub enum ChannelIdentifier {
    Order(i16),
    Name(String),
    Label(String),
}

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

    #[inline]
    pub fn id(&self) -> u16 {
        self.id
    }
    #[inline]
    pub fn acquisition_id(&self) -> u16 {
        self.acquisition_id
    }
    #[inline]
    pub fn name(&self) -> &str {
        &self.channel_name
    }
    #[inline]
    pub fn order_number(&self) -> i16 {
        self.order_number
    }
    #[inline]
    pub fn label(&self) -> &str {
        &self.channel_label
    }
}
