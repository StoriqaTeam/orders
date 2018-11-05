use failure::Error as FailureError;
use future::{self, Future};
use stq_acl::{AclEngine, Verdict};

pub struct OrdersAcl<F>(pub F);

impl<F, Context> AclEngine<Context, FailureError> for OrdersAcl<F>
where
    F: Fn(&mut Context) -> bool,
    Context: 'static,
{
    fn ensure_access(&self, ctx: Context) -> Box<Future<Item = Context, Error = (FailureError, Context)>> {
        Box::new(self.allows(ctx).and_then(|(allowed, ctx)| {
            future::result(if allowed {
                Ok(ctx)
            } else {
                Err((FailureError::from(::errors::Error::Forbidden), ctx))
            })
        }))
    }

    fn allows(&self, mut ctx: Context) -> Verdict<Context, FailureError> {
        let allowed = (self.0)(&mut ctx);
        Box::new(future::ok((allowed, ctx)))
    }
}
