module Main exposing (GameModel, Msg, update, view, subscriptions, init)

import Css exposing (asPairs, backgroundColor, maxHeight, maxWidth, minHeight, minWidth, px, vw)
import Css.Colors exposing (aqua, gray, silver)
import Html exposing (..)
import Html.Attributes exposing (src, style)
import Html.Events exposing (onClick)
import Json.Decode exposing (Decoder, decodeString)
import Json.Decode.Pipeline exposing (decode, required)
import WebSocket


main : Program Never Model Msg
main =
    Html.program
        { init = init
        , view = view
        , update = update
        , subscriptions = subscriptions
        }



-- model


colourDecoder : Decoder PieceColour
colourDecoder =
    Json.Decode.andThen
        (\x ->
            case x of
                "White" ->
                    Json.Decode.succeed White

                "Black" ->
                    Json.Decode.succeed Black

                _ ->
                    Json.Decode.fail "Invalid piece colour"
        )
        Json.Decode.string


type PieceColour
    = White
    | Black


kindDecoder : Decoder PieceKind
kindDecoder =
    Json.Decode.andThen
        (\x ->
            case x of
                "King" ->
                    Json.Decode.succeed King

                "Queen" ->
                    Json.Decode.succeed Queen

                "Bishop" ->
                    Json.Decode.succeed Bishop

                "Knight" ->
                    Json.Decode.succeed Knight

                "Rook" ->
                    Json.Decode.succeed Rook

                "Pawn" ->
                    Json.Decode.succeed Pawn

                _ ->
                    Json.Decode.fail "Invalid piece type"
        )
        Json.Decode.string


type PieceKind
    = King
    | Queen
    | Bishop
    | Knight
    | Rook
    | Pawn


pieceDecoder : Decoder Piece
pieceDecoder =
    decode Piece
        |> required "kind" kindDecoder
        |> required "colour" colourDecoder


type alias Piece =
    { kind : PieceKind
    , colour : PieceColour
    }


type ClickState
    = Unselected
    | Selected Int Int
    | Done


boardDecoder : Decoder (List (List (Maybe Piece)))
boardDecoder =
    Json.Decode.andThen
        (\x ->
            if
                List.length x
                    == 8
                    && List.all
                        (\x -> List.length x == 8)
                        x
            then
                Json.Decode.succeed x
            else
                Json.Decode.fail "Invalid board dimensions"
        )
        (Json.Decode.list
            (Json.Decode.list (Json.Decode.nullable pieceDecoder))
        )


type alias NetworkMessage =
    { board : List (List (Maybe Piece)), turn : PieceColour }


messageDecoder : Decoder NetworkMessage
messageDecoder =
    decode NetworkMessage
        |> required "board" boardDecoder
        |> required "turn" colourDecoder


type alias GameModel =
    { board : List (List (Maybe Piece))
    , self : PieceColour
    , turn : PieceColour
    , clickState : ClickState
    , url : String
    }


type Model
    = SelectingTeam
    | Loading PieceColour String
    | InGame GameModel



-- update and messages


type Msg
    = Chosen PieceColour
    | Click Int Int
    | Unclick
    | Transmission String


update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case model of
        SelectingTeam ->
            (case msg of
                Chosen x ->
                    ( Loading x "ws://127.0.0.1:2828", Cmd.none )

                _ ->
                    ( model, Cmd.none )
            )

        Loading team url ->
            (case msg of
                Transmission msg ->
                    (case decodeString messageDecoder msg of
                        Ok update ->
                            ( InGame
                                { board = update.board
                                , turn = update.turn
                                , url = url
                                , self = team
                                , clickState = Unselected
                                }
                            , Cmd.none
                            )

                        _ ->
                            -- error handling is for weenies part 2
                            ( Loading team url, Cmd.none )
                    )

                _ ->
                    ( model, Cmd.none )
            )

        InGame model ->
            (case msg of
                Click x y ->
                    if model.turn == model.self then
                        (case model.clickState of
                            Unselected ->
                                ( InGame { model | clickState = Selected x y }, Cmd.none )

                            Selected x0 y0 ->
                                ( InGame { model | clickState = Done }
                                , WebSocket.send model.url
                                    (let
                                        ( old, new ) =
                                            case model.self of
                                                White ->
                                                    ( [ x0, y0 ], [ x, y ] )

                                                Black ->
                                                    ( [ 7 - x0, 7 - y0 ], [ 7 - x, 7 - y ] )
                                     in
                                        ("{\"action\": {\"from\": " ++ toString old ++ ", \"to\": " ++ toString new ++ "}, \"weight\": 1}")
                                    )
                                )

                            _ ->
                                ( InGame model, Cmd.none )
                        )
                    else
                        ( InGame model, Cmd.none )

                Unclick ->
                    ( InGame { model | clickState = Unselected }, Cmd.none )

                Transmission msg ->
                    (case decodeString messageDecoder msg of
                        Ok update ->
                            let
                                newClickState =
                                    if update.turn == model.self then
                                        (case model.clickState of
                                            Selected x y ->
                                                Selected x y

                                            _ ->
                                                Unselected
                                        )
                                    else
                                        Unselected
                            in
                                ( InGame { model | board = update.board, turn = update.turn, clickState = newClickState }, Cmd.none )

                        _ ->
                            -- error handling is for weenies
                            ( InGame model, Cmd.none )
                    )

                _ ->
                    ( InGame model, Cmd.none )
            )



-- view


styles : List Css.Style -> Attribute msg
styles =
    asPairs >> style


renderPiece : Piece -> String
renderPiece piece =
    case piece.kind of
        King ->
            (case piece.colour of
                Black ->
                    "https://upload.wikimedia.org/wikipedia/commons/e/e3/Chess_kdt60.png"

                White ->
                    "https://upload.wikimedia.org/wikipedia/commons/3/3b/Chess_klt60.png"
            )

        Queen ->
            (case piece.colour of
                Black ->
                    "https://upload.wikimedia.org/wikipedia/commons/a/af/Chess_qdt60.png"

                White ->
                    "https://upload.wikimedia.org/wikipedia/commons/4/49/Chess_qlt60.png"
            )

        Bishop ->
            (case piece.colour of
                Black ->
                    "https://upload.wikimedia.org/wikipedia/commons/8/81/Chess_bdt60.png"

                White ->
                    "https://upload.wikimedia.org/wikipedia/commons/9/9b/Chess_blt60.png"
            )

        Knight ->
            (case piece.colour of
                Black ->
                    "https://upload.wikimedia.org/wikipedia/commons/f/f1/Chess_ndt60.png"

                White ->
                    "https://upload.wikimedia.org/wikipedia/commons/2/28/Chess_nlt60.png"
            )

        Rook ->
            (case piece.colour of
                Black ->
                    "https://upload.wikimedia.org/wikipedia/commons/a/a0/Chess_rdt60.png"

                White ->
                    "https://upload.wikimedia.org/wikipedia/commons/5/5c/Chess_rlt60.png"
            )

        Pawn ->
            (case piece.colour of
                Black ->
                    "https://upload.wikimedia.org/wikipedia/commons/c/cd/Chess_pdt60.png"

                White ->
                    "https://upload.wikimedia.org/wikipedia/commons/0/04/Chess_plt60.png"
            )


renderSquare : Int -> Int -> Bool -> Maybe Piece -> Html Msg
renderSquare x y highlighted elem =
    let
        base =
            if highlighted then
                aqua
            else if (x + y) % 2 == 1 then
                gray
            else
                silver

        piece =
            Maybe.withDefault [] (Maybe.map (renderPiece >> src >> (\x -> img [ x ] []) >> List.singleton) elem)

        event =
            if highlighted then
                Unclick
            else
                (Click x y)
    in
        td []
            [ button
                [ onClick event
                , styles
                    [ backgroundColor base
                    , minHeight (vw 8)
                    , maxHeight (vw 8)
                    , minWidth (vw 8)
                    , maxWidth (vw 8)
                    ]
                ]
                piece
            ]


renderBoard : Int -> Int -> List (List (Maybe Piece)) -> Html Msg
renderBoard selectedX selectedY board =
    table []
        (board
            |> List.indexedMap
                (\y row ->
                    row
                        |> List.indexedMap (\x piece -> renderSquare x y (x == selectedX && y == selectedY) piece)
                        |> tr []
                )
        )


viewGame : GameModel -> Html Msg
viewGame model =
    let
        ( x, y ) =
            case model.clickState of
                Unselected ->
                    ( -1, -1 )

                Done ->
                    ( -1, -1 )

                Selected x y ->
                    ( x, y )

        board =
            case model.self of
                White ->
                    renderBoard x y model.board

                Black ->
                    renderBoard x y (model.board |> List.reverse |> List.map List.reverse)

        header =
            p []
                [ text
                    (case model.turn of
                        White ->
                            "It is white's turn."

                        Black ->
                            "It is black's turn."
                    )
                ]

        has_moved =
            p []
                [ text
                    (case model.clickState of
                        Unselected ->
                            if model.turn == model.self then
                                "Please make a move."
                            else
                                "It is not your turn."

                        Selected _ _ ->
                            ""

                        Done ->
                            "You have made your move. Please wait for the next turn."
                    )
                ]
    in
        div [ styles [ Css.padding (vw 5) ] ]
            [ div [ styles [ Css.padding (px 20) ] ] [ header, has_moved ]
            , board
            ]


view : Model -> Html Msg
view model =
    case model of
        SelectingTeam ->
            let
                buttonStyle =
                    styles [ Css.padding (px 50), Css.margin (px 50) ]
            in
                div []
                    [ button
                        [ onClick (Chosen White), buttonStyle ]
                        [ text "I want to play for the white team" ]
                    , button [ onClick (Chosen Black), buttonStyle ] [ text "I want to play for the black team" ]
                    ]

        Loading _ _ ->
            text "Loading"

        InGame m ->
            viewGame m



-- subscriptions


subscriptions : Model -> Sub Msg
subscriptions model =
    case model of
        SelectingTeam ->
            Sub.none

        Loading _ url ->
            WebSocket.listen url Transmission

        InGame model ->
            WebSocket.listen model.url Transmission



-- init


initBoard : List (List (Maybe Piece))
initBoard =
    List.concat
        [ [ List.map (\x -> Just ({ kind = x, colour = Black }))
                [ Rook
                , Knight
                , Bishop
                , Queen
                , King
                , Bishop
                , Knight
                , Rook
                ]
          , List.repeat 8 (Just ({ kind = Pawn, colour = Black }))
          ]
        , List.repeat 4 (List.repeat 8 Nothing)
        , [ List.repeat 8 (Just ({ kind = Pawn, colour = White }))
          , List.map (\x -> Just ({ kind = x, colour = White }))
                [ Rook
                , Knight
                , Bishop
                , Queen
                , King
                , Bishop
                , Knight
                , Rook
                ]
          ]
        ]


init : ( Model, Cmd Msg )
init =
    ( SelectingTeam
    , Cmd.none
    )
