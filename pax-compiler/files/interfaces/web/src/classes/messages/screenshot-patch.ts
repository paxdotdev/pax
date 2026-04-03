export class ScreenshotPatch {
    public id?: number;
    public scale?: number;

    fromPatch(jsonMessage: any) {
        this.id = jsonMessage["id"];
        this.scale = jsonMessage["scale"];
    }

    cleanUp(){
        this.id = undefined;
        this.scale = undefined;
    }
}
