export class ScreenshotPatch {
    public id?: number;

    fromPatch(jsonMessage: any) {
        this.id = jsonMessage["id"];
    }

    cleanUp(){
        this.id = undefined;
    }
}
