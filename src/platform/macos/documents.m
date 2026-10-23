#import <AppKit/AppKit.h>
#import <Carbon/Carbon.h>
#include <stdbool.h>

typedef bool (*SilicateDocumentCallback)(const char *);

@interface SilicateDocumentEvents : NSObject
@property(nonatomic, assign) SilicateDocumentCallback callback;
@end

@implementation SilicateDocumentEvents
- (void)willFinishLaunching:(NSNotification *)notification {
    // Keep winit's application delegate and lifecycle handling intact.
    [[NSAppleEventManager sharedAppleEventManager]
        setEventHandler:self
        andSelector:@selector(openDocuments:reply:)
        forEventClass:kCoreEventClass
        andEventID:kAEOpenDocuments];
}

- (void)openDocuments:(NSAppleEventDescriptor *)event
                reply:(NSAppleEventDescriptor *)reply {
    NSAppleEventDescriptor *items = [event paramDescriptorForKeyword:keyDirectObject];
    BOOL accepted = items.numberOfItems > 0;
    for (NSInteger index = 1; index <= items.numberOfItems; index++) {
        NSURL *url = [items descriptorAtIndex:index].fileURLValue;
        if (!url.isFileURL || !self.callback(url.fileSystemRepresentation)) {
            accepted = NO;
        }
    }
    if (!accepted) {
        [reply setParamDescriptor:[NSAppleEventDescriptor descriptorWithInt32:errAEEventNotHandled]
                      forKeyword:keyErrorNumber];
    }
}
@end

void silicate_listen_for_documents(SilicateDocumentCallback callback) {
    // AppKit retains neither the event target nor an application-owned lifecycle here.
    // This listener intentionally lives for the application's process lifetime.
    static SilicateDocumentEvents *listener;
    listener = [SilicateDocumentEvents new];
    listener.callback = callback;
    [[NSNotificationCenter defaultCenter] addObserver:listener
        selector:@selector(willFinishLaunching:)
        name:NSApplicationWillFinishLaunchingNotification object:nil];
}
