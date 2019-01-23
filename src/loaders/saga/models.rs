#[derive(Debug, Serialize, Clone, Copy, Eq, PartialEq, Hash)]
pub struct OrderPaymentStateRequest {
    pub state: PaymentState,
}

#[derive(Debug, Serialize, Clone, Copy, Eq, PartialEq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum PaymentState {
    /// Order created and maybe paid by customer
    Initial,
    /// Store manager declined the order
    Declined,
    /// Store manager confirmed the order, money was captured
    Captured,
    /// Need money refund to customer
    RefundNeeded,
    /// Money was refunded to customer
    Refunded,
    /// Money was paid to seller
    PaidToSeller,
    /// Need money payment to seller
    PaymentToSellerNeeded,
}
