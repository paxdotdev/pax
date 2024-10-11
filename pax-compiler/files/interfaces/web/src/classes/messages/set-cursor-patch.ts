export class SetCursorPatch {
    public cursor?: string;

    fromPatch(jsonMessage: any) {
        this.cursor = jsonMessage["cursor"];
    }

    cleanUp(){
        this.cursor = undefined;
    }
}
